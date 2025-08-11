mod user_groups;

use std::{collections::HashSet, path::Path};

use rusqlite::{
    Connection, Error,
    types::{FromSql, ToSql},
};
use std::collections::HashMap;
use time::OffsetDateTime;

use crate::{
    constants::{MEMORY_MAX_MESSAGES, MEMORY_RETENTION},
    memory::user_groups::{GroupInfo, GroupSets},
};

const MEMORY_DB_NAME: &str = "ai_memory.db";

#[derive(Debug, PartialEq, Clone)]
pub enum Sender {
    User,
    Assistant,
}

impl std::fmt::Display for Sender {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Sender::User => write!(f, "user"),
            Sender::Assistant => write!(f, "assistant"),
        }
    }
}

impl ToSql for Sender {
    fn to_sql(&self) -> rusqlite::Result<rusqlite::types::ToSqlOutput<'_>> {
        Ok(self.to_string().into())
    }
}

impl FromSql for Sender {
    fn column_result(value: rusqlite::types::ValueRef) -> rusqlite::types::FromSqlResult<Self> {
        match value.as_str()? {
            "user" => Ok(Sender::User),
            "assistant" => Ok(Sender::Assistant),
            _ => Err(rusqlite::types::FromSqlError::Other(
                "Invalid Sender value".into(),
            )),
        }
    }
}

#[derive(Debug, PartialEq, Clone)]
struct Entry {
    sender: Sender,
    receiver: String, // The channel that received the message
    timestamp: OffsetDateTime,
    message: String,
}

pub struct Memory {
    entries: HashMap<String, Vec<Entry>>,
    joined_users: GroupSets,
    sqlite: Connection,
}

fn load_history(connection: &mut Connection) -> Result<HashMap<String, Vec<Entry>>, Error> {
    let mut entries = {
        let mut load_entries = connection.prepare(
            "SELECT user, sender, receiver, timestamp, message FROM memory ORDER BY timestamp",
        )?;
        let entry_iter = load_entries.query_map([], |row| {
            Ok((
                row.get("user")?,
                Entry {
                    sender: row.get("sender")?,
                    receiver: row.get("receiver")?,
                    timestamp: row.get("timestamp")?,
                    message: row.get("message")?,
                },
            ))
        })?;
        let mut entries = HashMap::new();
        for entry in entry_iter {
            let (user, entry) = entry?;
            entries.entry(user).or_insert_with(Vec::new).push(entry);
        }
        entries
    };
    // Remove expired entries
    let oldest_allowed = OffsetDateTime::now_utc() - MEMORY_RETENTION;
    entries.retain(|_, user_entries| {
        user_entries.retain(|entry| entry.timestamp > oldest_allowed);
        !user_entries.is_empty()
    });
    // Remove entries from the front so that only up to MEMORY_MAX_MESSAGES remain
    for user_entries in entries.values_mut() {
        if user_entries.len() > MEMORY_MAX_MESSAGES {
            user_entries.drain(0..user_entries.len() - MEMORY_MAX_MESSAGES);
        }
    }
    Ok(entries)
}

fn load_group_sets(connection: &mut Connection) -> Result<GroupSets, Error> {
    struct Row {
        user: String,
        group_id: i64,
        last_modified: OffsetDateTime,
    }
    let mut load_group_sets =
        connection.prepare("SELECT user_name, group_id, last_modified FROM group_sets")?;
    let group_set_iter = load_group_sets.query_map([], |row| {
        Ok(Row {
            user: row.get("user_name")?,
            group_id: row.get("group_id")?,
            last_modified: row.get("last_modified")?,
        })
    })?;

    let mut user_to_group: HashMap<String, usize> = HashMap::new();
    let mut group_map: HashMap<usize, GroupInfo> = HashMap::new();

    for row in group_set_iter {
        let row = row?;
        let group_id = row.group_id as usize;

        let group_info = group_map.entry(group_id).or_insert_with(|| GroupInfo {
            members: HashSet::new(),
            last_modified: row.last_modified,
        });

        group_info.members.insert(row.user.clone());
        user_to_group.insert(row.user, group_id);
    }
    Ok(GroupSets::from_maps(user_to_group, group_map))
}

impl Memory {
    pub fn new_from_path(config_path: &Path) -> Result<Self, String> {
        let mut connection = Connection::open(config_path.join(MEMORY_DB_NAME))
            .map_err(|e| format!("Failed to open memory DB: {}", e))?;

        let create_db = "CREATE TABLE IF NOT EXISTS memory (user TEXT NOT NULL, sender TEXT NOT NULL, receiver TEXT NOT NULL, timestamp TEXT NOT NULL, message TEXT NOT NULL)";
        connection
            .execute(create_db, ())
            .map_err(|e| e.to_string())?;

        let create_group_sets_table = "CREATE TABLE IF NOT EXISTS group_sets (user_name TEXT NOT NULL, group_id INTEGER NOT NULL, last_modified TEXT NOT NULL)";
        connection
            .execute(create_group_sets_table, ())
            .map_err(|e| e.to_string())?;

        Ok(Self {
            entries: load_history(&mut connection).map_err(|e| e.to_string())?,
            joined_users: load_group_sets(&mut connection).map_err(|e| e.to_string())?,
            sqlite: connection,
        })
    }

    pub fn save(&mut self) -> Result<(), String> {
        let tx = self.sqlite.transaction().map_err(|e| e.to_string())?;
        tx.execute("DELETE FROM memory", [])
            .map_err(|e| e.to_string())?;
        {
            let mut stmt = tx
                .prepare("INSERT INTO memory (user, sender, receiver, timestamp, message) VALUES (?1, ?2, ?3, ?4, ?5)")
                .map_err(|e| e.to_string())?;
            for (user, entries) in &self.entries {
                for entry in entries {
                    stmt.execute((
                        user,
                        entry.sender.clone(),
                        entry.receiver.clone(),
                        entry.timestamp,
                        entry.message.clone(),
                    ))
                    .map_err(|e| e.to_string())?;
                }
            }
        }

        tx.execute("DELETE FROM group_sets", [])
            .map_err(|e| e.to_string())?;
        {
            let mut stmt = tx
                .prepare("INSERT INTO group_sets (user_name, group_id, last_modified) VALUES (?1, ?2, ?3)")
                .map_err(|e| e.to_string())?;
            for (user, group_id) in self.joined_users.get_user_to_group_mappings().iter() {
                if let Some(group_info) = self.joined_users.get_groups().get(group_id) {
                    stmt.execute((user, *group_id as i64, group_info.last_modified))
                        .map_err(|e| e.to_string())?;
                }
            }
        }

        tx.commit().map_err(|e| e.to_string())?;
        Ok(())
    }

    // Add to the history of the given user.
    pub fn add_to_history(&mut self, user: &str, sender: Sender, receiver: &str, message: &str) {
        self.entries
            .entry(user.to_string())
            .or_default()
            .push(Entry {
                sender,
                receiver: receiver.to_string(),
                timestamp: OffsetDateTime::now_utc(),
                message: message.to_string(),
            });
    }

    // Clear the history of the given user.
    pub fn clear_history(&mut self, user: &str, receiver: &str) {
        if let Some(entries) = self.entries.get_mut(user) {
            entries.retain(|entry| entry.receiver != receiver);
            if entries.is_empty() {
                self.entries.remove(user);
            }
        }
    }

    // Returns matching messages in chronological order with timestamps.
    pub fn user_history(
        &self,
        user: &str,
        receiver: &str,
    ) -> Vec<(Sender, String, OffsetDateTime)> {
        self.entries.get(user).map_or_else(Vec::new, |entries| {
            entries
                .iter()
                .filter(|entry| entry.receiver == receiver)
                .map(|entry| (entry.sender.clone(), entry.message.clone(), entry.timestamp))
                .collect()
        })
    }

    // Join two users so they share memory
    pub fn join_users(&mut self, user1: &str, user2: &str) {
        self.joined_users.union(user1, user2);
    }

    // Remove a user from their joined group, making them solo
    pub fn make_user_solo(&mut self, user: &str) {
        self.joined_users.remove_user(user);
    }

    // Get all users joined with the given user including the user themselves
    pub fn get_joined_users(&self, user: &str) -> Vec<String> {
        let mut users = self.joined_users.get_group_members(user);
        users.sort();
        users
    }

    // Get all users joined with the given user excluding the user themselves
    pub fn get_joined_users_excluding_self(&self, user: &str) -> Vec<String> {
        let mut users = self
            .joined_users
            .get_group_members(user)
            .into_iter()
            .filter(|u| u != user)
            .collect::<Vec<String>>();
        users.sort();
        users
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::Connection;
    use tempfile::tempdir;

    fn setup_memory() -> (tempfile::TempDir, Memory) {
        let dir = tempdir().unwrap();
        let memory = Memory::new_from_path(dir.path()).unwrap();
        (dir, memory)
    }

    #[test]
    fn test_memory_new_from_path_and_save() {
        let (dir, mut memory) = setup_memory();

        memory.add_to_history("user1", Sender::User, "receiver1", "message1");
        memory.add_to_history("user1", Sender::Assistant, "receiver1", "message2");
        memory.add_to_history("user2", Sender::User, "receiver2", "messageA");
        memory.save().unwrap();

        let loaded_memory = Memory::new_from_path(dir.path()).unwrap();
        let user1_history = loaded_memory.user_history("user1", "receiver1");
        let user2_history = loaded_memory.user_history("user2", "receiver2");

        assert_eq!(user1_history.len(), 2);
        assert_eq!(user2_history.len(), 1);
        assert_eq!(user1_history[0].1, "message1");
        assert_eq!(user1_history[1].1, "message2");
        assert_eq!(user2_history[0].1, "messageA");
    }

    #[test]
    fn test_memory_clear_history() {
        let (dir, mut memory) = setup_memory();

        memory.add_to_history("user1", Sender::User, "receiver1", "message1");
        memory.add_to_history("user2", Sender::User, "receiver2", "messageA");
        memory.save().unwrap();

        memory.clear_history("user1", "receiver1");
        memory.save().unwrap();

        let loaded_memory = Memory::new_from_path(dir.path()).unwrap();
        assert!(loaded_memory.user_history("user1", "receiver1").is_empty());
        assert_eq!(loaded_memory.user_history("user2", "receiver2").len(), 1);
    }

    #[test]
    fn test_memory_load_removes_old_entries() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join(MEMORY_DB_NAME);
        let connection = Connection::open(&db_path).unwrap();
        connection.execute("CREATE TABLE IF NOT EXISTS memory (user TEXT NOT NULL, sender TEXT NOT NULL, receiver TEXT NOT NULL, timestamp TEXT NOT NULL, message TEXT NOT NULL)", ()).unwrap();

        let now = OffsetDateTime::now_utc();
        let old_time = now - (MEMORY_RETENTION + time::Duration::seconds(1));
        let recent_time = now - (MEMORY_RETENTION - time::Duration::seconds(1));

        for (user, sender, receiver, timestamp, message) in [
            ("u1", Sender::User, "r1", old_time, "old_message"),
            ("u1", Sender::Assistant, "r1", recent_time, "recent_message"),
            ("u1", Sender::User, "r2", recent_time, "recent_2"),
        ] {
            connection.execute(
                "INSERT INTO memory (user, sender, receiver, timestamp, message) VALUES (?1, ?2, ?3, ?4, ?5)",
                (user, sender, receiver, timestamp, message),
            ).unwrap();
        }

        let memory = Memory::new_from_path(dir.path()).unwrap();
        let user1_history = memory.user_history("u1", "r1");
        let user2_history = memory.user_history("u1", "r2");
        assert_eq!(user1_history.len(), 1);
        assert_eq!(user1_history[0].1, "recent_message");
        assert_eq!(user2_history.len(), 1);
        assert_eq!(user2_history[0].1, "recent_2");
    }

    #[test]
    fn test_memory_max_messages() {
        let (dir, mut memory) = setup_memory();

        for i in 0..(MEMORY_MAX_MESSAGES + 5) {
            memory.add_to_history("user1", Sender::User, "receiver1", &format!("msg{}", i));
        }
        memory.save().unwrap();

        let loaded_memory = Memory::new_from_path(dir.path()).unwrap();
        let history = loaded_memory.user_history("user1", "receiver1");

        assert_eq!(history.len(), MEMORY_MAX_MESSAGES);
        // The oldest messages should have been dropped, so the first message should be msg5
        assert_eq!(history[0].1, "msg5");
        assert_eq!(
            history[MEMORY_MAX_MESSAGES - 1].1,
            format!("msg{}", MEMORY_MAX_MESSAGES + 4)
        );
    }

    #[test]
    fn test_memory_joined_users_persistence() {
        let (dir, mut memory) = setup_memory();

        memory.add_to_history("user1", Sender::User, "receiver1", "message1");
        memory.add_to_history("user2", Sender::User, "receiver2", "message2");
        memory.join_users("user1", "user2");
        memory.save().unwrap();

        let loaded_memory = Memory::new_from_path(dir.path()).unwrap();
        let joined_users = loaded_memory.get_joined_users("user1");

        assert_eq!(joined_users.len(), 2);
        assert!(joined_users.contains(&"user1".to_string()));
        assert!(joined_users.contains(&"user2".to_string()));
    }
}
