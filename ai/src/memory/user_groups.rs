use std::collections::{HashMap, HashSet};
use time::OffsetDateTime;

use crate::constants::USER_GROUP_RETENTION;

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

#[derive(Debug, Default)]
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
        mut user_to_group: HashMap<String, usize>,
        mut groups: HashMap<usize, GroupInfo>,
    ) -> Self {
        // Consistency check: Remove users from user_to_group if their group doesn't exist
        user_to_group.retain(|_user, group_id| groups.contains_key(group_id));

        // Consistency check: Ensure all users in groups are present in user_to_group
        // and vice versa. Handle users appearing in multiple groups by keeping them
        // in the first group encountered and removing from others.
        let mut users_in_groups: HashSet<String> = HashSet::new();
        let mut user_assignments: HashMap<String, usize> = HashMap::new();

        // First pass: collect all users and their first group assignment
        for (group_id, group_info) in &groups {
            for user in &group_info.members {
                users_in_groups.insert(user.clone());
                // Only assign to first group encountered
                user_assignments.entry(user.clone()).or_insert(*group_id);
            }
        }

        // Update user_to_group with the assignments
        for (user, group_id) in &user_assignments {
            user_to_group.insert(user.clone(), *group_id);
        }

        // Remove users from groups where they don't belong
        for (group_id, group_info) in groups.iter_mut() {
            group_info
                .members
                .retain(|user| user_assignments.get(user) == Some(group_id));
        }

        // Remove users from user_to_group that don't exist in any group
        user_to_group.retain(|user, _group_id| users_in_groups.contains(user));

        // Remove users from groups that don't exist in user_to_group
        for group_info in groups.values_mut() {
            group_info
                .members
                .retain(|user| user_to_group.contains_key(user));
        }

        // Remove empty groups
        groups.retain(|_group_id, group_info| !group_info.members.is_empty());

        let next_group_id = groups.keys().max().map(|x| x + 1).unwrap_or_default();
        Self {
            user_to_group,
            groups,
            next_group_id,
        }
    }

    pub fn expire_old_groups(&mut self, now: OffsetDateTime) {
        // Remove groups that are older than USER_GROUP_RETENTION.
        let oldest_allowed = now - USER_GROUP_RETENTION;
        self.groups
            .retain(|_, info| info.last_modified > oldest_allowed);
        self.user_to_group
            .retain(|_, group_id| self.groups.contains_key(group_id));

        // Remove singleton groups, they are redundant
        self.groups.iter().for_each(|(_, info)| {
            if info.members.len() == 1 {
                self.user_to_group
                    .remove(info.members.iter().next().unwrap());
            }
        });
        self.groups.retain(|_, info| info.members.len() > 1);
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
mod tests;
