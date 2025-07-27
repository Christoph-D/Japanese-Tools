use std::path::Path;

use rusqlite::{
    Connection, Error,
    types::{FromSql, ToSql},
};
use time::OffsetDateTime;

use crate::constants::{MEMORY_MAX_MESSAGES, MEMORY_RETENTION};

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
    entries: std::collections::HashMap<String, Vec<Entry>>,
    sqlite: Connection,
}

fn locate_db() -> Result<std::path::PathBuf, String> {
    let exe_path = std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|p| p.to_path_buf()));
    let paths = [exe_path.clone(), std::env::current_dir().ok()];
    for path in paths.iter().flatten() {
        let p = path.join(MEMORY_DB_NAME);
        if p.is_file() {
            return Ok(p);
        }
    }
    exe_path
        .map(|p| p.join(MEMORY_DB_NAME))
        .ok_or("Cannot find memory DB".to_string())
}

fn load(
    connection: &mut Connection,
) -> Result<std::collections::HashMap<String, Vec<Entry>>, Error> {
    let create_db = "CREATE TABLE IF NOT EXISTS memory (user TEXT NOT NULL, sender TEXT NOT NULL, receiver TEXT NOT NULL, timestamp TEXT NOT NULL, message TEXT NOT NULL)";
    connection.execute(create_db, ())?;

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
        let mut entries = std::collections::HashMap::new();
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
            let excess = user_entries.len() - MEMORY_MAX_MESSAGES;
            user_entries.drain(0..excess);
        }
    }
    Ok(entries)
}

impl Memory {
    pub fn new_from_disk() -> Result<Self, String> {
        Self::new_from_path(&locate_db()?)
    }

    fn new_from_path(db_path: &Path) -> Result<Self, String> {
        let mut connection =
            Connection::open(db_path).map_err(|e| format!("Failed to open memory DB: {}", e))?;
        Ok(Self {
            entries: load(&mut connection).map_err(|e| e.to_string())?,
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

    // Returns matching messages in chronological order.
    pub fn user_history(&self, user: &str, receiver: &str) -> Vec<(Sender, String)> {
        self.entries.get(user).map_or_else(Vec::new, |entries| {
            entries
                .iter()
                .filter(|entry| entry.receiver == receiver)
                .map(|entry| (entry.sender.clone(), entry.message.clone()))
                .collect()
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::Connection;
    use tempfile::tempdir;

    #[test]
    fn test_memory_new_from_path_and_save() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("test_memory.db");
        let mut memory = Memory::new_from_path(&db_path).unwrap();

        assert!(memory.entries.is_empty());

        memory.add_to_history("user1", Sender::User, "receiver1", "message1");
        memory.add_to_history("user1", Sender::Assistant, "receiver1", "message2");
        memory.add_to_history("user2", Sender::User, "receiver2", "messageA");

        memory.save().unwrap();

        let loaded_memory = Memory::new_from_path(&db_path).unwrap();
        assert_eq!(loaded_memory.user_history("user1", "receiver1").len(), 2);
        assert_eq!(loaded_memory.user_history("user2", "receiver2").len(), 1);
        assert_eq!(
            loaded_memory.user_history("user1", "receiver1")[0].1,
            "message1"
        );
        assert_eq!(
            loaded_memory.user_history("user1", "receiver1")[1].1,
            "message2"
        );
        assert_eq!(
            loaded_memory.user_history("user2", "receiver2")[0].1,
            "messageA"
        );
    }

    #[test]
    fn test_memory_clear_history() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("test_memory_clear.db");
        let mut memory = Memory::new_from_path(&db_path).unwrap();

        memory.add_to_history("user1", Sender::User, "receiver1", "message1");
        memory.add_to_history("user2", Sender::User, "receiver2", "messageA");
        memory.save().unwrap();

        memory.clear_history("user1", "receiver1");
        memory.save().unwrap();

        let loaded_memory = Memory::new_from_path(&db_path).unwrap();
        assert!(loaded_memory.user_history("user1", "receiver1").is_empty());
        assert_eq!(loaded_memory.user_history("user2", "receiver2").len(), 1);
    }

    #[test]
    fn test_memory_load_removes_old_entries() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("test_memory_old_entries.db");
        let connection = Connection::open(&db_path).unwrap();
        let create_db = "CREATE TABLE IF NOT EXISTS memory (user TEXT NOT NULL, sender TEXT NOT NULL, receiver TEXT NOT NULL, timestamp TEXT NOT NULL, message TEXT NOT NULL)";
        connection.execute(create_db, ()).unwrap();

        let now = OffsetDateTime::now_utc();
        let old_time = now - (MEMORY_RETENTION + time::Duration::seconds(1));
        let recent_time = now - (MEMORY_RETENTION - time::Duration::seconds(1));

        connection
            .execute(
                "INSERT INTO memory (user, sender, receiver, timestamp, message) VALUES (?1, ?2, ?3, ?4, ?5)",
                ("user1", Sender::User, "receiver1", old_time, "old_message"),
            )
            .unwrap();
        connection
            .execute(
                "INSERT INTO memory (user, sender, receiver, timestamp, message) VALUES (?1, ?2, ?3, ?4, ?5)",
                ("user1", Sender::Assistant, "receiver1", recent_time, "recent_message"),
            )
            .unwrap();
        connection
            .execute(
                "INSERT INTO memory (user, sender, receiver, timestamp, message) VALUES (?1, ?2, ?3, ?4, ?5)",
                ("user2", Sender::User, "receiver2", recent_time, "another_recent_message"),
            )
            .unwrap();

        let memory = Memory::new_from_path(&db_path).unwrap();
        assert_eq!(memory.user_history("user1", "receiver1").len(), 1);
        assert_eq!(
            memory.user_history("user1", "receiver1")[0].1,
            "recent_message"
        );
        assert_eq!(memory.user_history("user2", "receiver2").len(), 1);
        assert_eq!(
            memory.user_history("user2", "receiver2")[0].1,
            "another_recent_message"
        );
    }

    #[test]
    fn test_memory_max_messages() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("test_memory_max_messages.db");
        let mut memory = Memory::new_from_path(&db_path).unwrap();

        for i in 0..(MEMORY_MAX_MESSAGES + 5) {
            memory.add_to_history("user1", Sender::User, "receiver1", &format!("msg{}", i));
        }
        memory.save().unwrap();

        let loaded_memory = Memory::new_from_path(&db_path).unwrap();
        let history = loaded_memory.user_history("user1", "receiver1");
        assert_eq!(history.len(), MEMORY_MAX_MESSAGES);

        // The oldest messages should have been dropped, so the first message should be msg5
        assert_eq!(history[0].1, "msg5");
        assert_eq!(history[MEMORY_MAX_MESSAGES - 1].1, format!("msg{}", MEMORY_MAX_MESSAGES + 4));
    }
}
