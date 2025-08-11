use std::collections::{HashMap, HashSet};
use time::OffsetDateTime;

#[derive(Debug, Clone)]
pub struct GroupInfo {
    pub members: HashSet<String>,
    pub last_modified: OffsetDateTime,
}

impl GroupInfo {
    pub fn new(user: String) -> Self {
        let mut members = HashSet::new();
        members.insert(user);
        Self {
            members,
            last_modified: OffsetDateTime::now_utc(),
        }
    }

    pub fn merge(&mut self, other: GroupInfo) {
        self.members.extend(other.members);
        self.last_modified = OffsetDateTime::now_utc();
    }
}

#[derive(Debug)]
pub struct GroupSets {
    // Maps user -> group_id
    user_to_group: HashMap<String, usize>,
    // Maps group_id -> GroupInfo
    groups: HashMap<usize, GroupInfo>,
    next_group_id: usize,
}

impl GroupSets {
    #[cfg(test)]
    fn new() -> Self {
        Self {
            user_to_group: HashMap::new(),
            groups: HashMap::new(),
            next_group_id: 0,
        }
    }

    pub fn from_maps(
        user_to_group: HashMap<String, usize>,
        groups: HashMap<usize, GroupInfo>,
    ) -> Self {
        let next_group_id = groups.keys().max().map(|x| x + 1).unwrap_or_default();
        Self {
            user_to_group,
            groups,
            next_group_id,
        }
    }

    /// Find the group ID that a user belongs to
    pub fn find_group(&self, user: &str) -> Option<usize> {
        self.user_to_group.get(user).copied()
    }

    /// Add a user as a singleton group if they don't already exist
    pub fn add_user(&mut self, user: &str) {
        if self.user_to_group.contains_key(user) {
            return;
        }

        let group_id = self.next_group_id;
        self.next_group_id += 1;

        let group_info = GroupInfo::new(user.to_string());
        self.groups.insert(group_id, group_info);
        self.user_to_group.insert(user.to_string(), group_id);
    }

    /// Merge the groups containing user1 and user2
    pub fn union(&mut self, user1: &str, user2: &str) {
        // Ensure both users exist in the union-set
        self.add_user(user1);
        self.add_user(user2);

        let group1_id = self.find_group(user1).unwrap();
        let group2_id = self.find_group(user2).unwrap();

        if group1_id == group2_id {
            return;
        }

        // Remove the second group and merge its members into the first group
        let group2 = self.groups.remove(&group2_id).unwrap();

        // Update all members of group2 to point to group1
        for member in &group2.members {
            self.user_to_group.insert(member.clone(), group1_id);
        }
        self.groups.get_mut(&group1_id).unwrap().merge(group2);
    }

    /// Get all users in the same group as the given user
    pub fn get_group_members(&self, user: &str) -> Vec<String> {
        match self.find_group(user) {
            Some(group_id) => self
                .groups
                .get(&group_id)
                .unwrap()
                .members
                .iter()
                .cloned()
                .collect(),
            None => vec![user.to_string()],
        }
    }

    /// Remove a user from their group, making them a singleton
    pub fn remove_user(&mut self, user: &str) {
        if let Some(group_id) = self.find_group(user) {
            // Remove user from their current group
            if let Some(group_info) = self.groups.get_mut(&group_id) {
                group_info.members.remove(user);
                group_info.last_modified = OffsetDateTime::now_utc();

                // If the group becomes empty, remove it entirely
                if group_info.members.is_empty() {
                    self.groups.remove(&group_id);
                }
            }

            // Remove the user from the mapping
            self.user_to_group.remove(user);

            // Add the user back as a singleton
            self.add_user(user);
        }
    }

    pub fn get_user_to_group_mappings(&self) -> &HashMap<String, usize> {
        &self.user_to_group
    }

    pub fn get_groups(&self) -> &HashMap<usize, GroupInfo> {
        &self.groups
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_group_set() {
        let group_set = GroupSets::new();
        assert_eq!(group_set.user_to_group.len(), 0);
        assert_eq!(group_set.groups.len(), 0);
        assert_eq!(group_set.next_group_id, 0);
    }

    #[test]
    fn test_add_user() {
        let mut group_set = GroupSets::new();

        group_set.add_user("alice");
        assert_eq!(group_set.find_group("alice"), Some(0));
        assert_eq!(group_set.get_group_members("alice"), vec!["alice"]);

        group_set.add_user("bob");
        assert_eq!(group_set.find_group("bob"), Some(1));
        assert_eq!(group_set.get_group_members("bob"), vec!["bob"]);

        // Adding the same user again should not create a new group
        group_set.add_user("alice");
        assert_eq!(group_set.find_group("alice"), Some(0));
        assert_eq!(group_set.groups.len(), 2);
    }

    #[test]
    fn test_union_different_groups() {
        let mut group_set = GroupSets::new();

        group_set.add_user("alice");
        group_set.add_user("bob");

        // Initially in different groups
        assert_ne!(group_set.find_group("alice"), group_set.find_group("bob"));

        // Union them
        group_set.union("alice", "bob");

        // Now they should be in the same group
        assert_eq!(group_set.find_group("alice"), group_set.find_group("bob"));

        let alice_members = group_set.get_group_members("alice");
        let bob_members = group_set.get_group_members("bob");
        assert_eq!(alice_members, bob_members);
        assert_eq!(alice_members.len(), 2);
        assert!(alice_members.contains(&"alice".to_string()));
        assert!(alice_members.contains(&"bob".to_string()));
    }

    #[test]
    fn test_union_same_group() {
        let mut group_set = GroupSets::new();

        group_set.add_user("alice");
        group_set.add_user("bob");
        group_set.union("alice", "bob");

        let group_id_before = group_set.find_group("alice");
        let members_before = group_set.get_group_members("alice");

        // Union again - should be no-op
        group_set.union("alice", "bob");

        assert_eq!(group_set.find_group("alice"), group_id_before);
        assert_eq!(group_set.get_group_members("alice"), members_before);
    }

    #[test]
    fn test_union_new_users() {
        let mut group_set = GroupSets::new();

        // Union two users that don't exist yet
        group_set.union("alice", "bob");

        // They should both exist and be in the same group
        assert_eq!(group_set.find_group("alice"), group_set.find_group("bob"));
        assert_eq!(group_set.get_group_members("alice").len(), 2);
    }

    #[test]
    fn test_remove_user() {
        let mut group_set = GroupSets::new();

        group_set.add_user("alice");
        group_set.add_user("bob");
        group_set.add_user("charlie");
        group_set.union("alice", "bob");
        group_set.union("bob", "charlie");

        // All three should be in the same group
        let group_id = group_set.find_group("alice");
        assert_eq!(group_set.find_group("bob"), group_id);
        assert_eq!(group_set.find_group("charlie"), group_id);
        assert_eq!(group_set.get_group_members("alice").len(), 3);

        // Remove alice
        group_set.remove_user("alice");

        // Alice should be in a new singleton group
        assert_ne!(group_set.find_group("alice"), group_id);
        assert_eq!(group_set.get_group_members("alice"), vec!["alice"]);

        // Bob and charlie should still be together
        assert_eq!(group_set.find_group("bob"), group_set.find_group("charlie"));
        assert_eq!(group_set.get_group_members("bob").len(), 2);
    }

    #[test]
    fn test_complex_unions() {
        let mut group_set = GroupSets::new();

        // Create multiple separate groups
        group_set.union("alice", "bob");
        group_set.union("charlie", "david");
        group_set.union("eve", "frank");

        // Verify separate groups
        assert_eq!(group_set.get_group_members("alice").len(), 2);
        assert_eq!(group_set.get_group_members("charlie").len(), 2);
        assert_eq!(group_set.get_group_members("eve").len(), 2);

        // Merge two groups
        group_set.union("alice", "charlie");

        // Alice, bob, charlie, david should all be together
        assert_eq!(group_set.get_group_members("alice").len(), 4);
        assert_eq!(group_set.find_group("alice"), group_set.find_group("bob"));
        assert_eq!(
            group_set.find_group("alice"),
            group_set.find_group("charlie")
        );
        assert_eq!(group_set.find_group("alice"), group_set.find_group("david"));

        // Eve and frank should still be separate
        assert_eq!(group_set.get_group_members("eve").len(), 2);
        assert_ne!(group_set.find_group("alice"), group_set.find_group("eve"));
    }
}
