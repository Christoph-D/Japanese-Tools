use super::*;
use proptest::prelude::*;

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

#[test]
fn test_from_maps_consistency_checks() {
    // Test case 1: user_to_group references non-existent group
    let mut user_to_group = HashMap::new();
    user_to_group.insert("alice".to_string(), 0);
    user_to_group.insert("bob".to_string(), 999); // Non-existent group

    let mut groups = HashMap::new();
    let mut alice_members = HashSet::new();
    alice_members.insert("alice".to_string());
    groups.insert(
        0,
        GroupInfo {
            members: alice_members,
            last_modified: OffsetDateTime::now_utc(),
        },
    );

    let group_set = GroupSets::from_maps(user_to_group, groups);

    // Bob should be removed from user_to_group since group 999 doesn't exist
    assert_eq!(group_set.user_to_group.len(), 1);
    assert!(group_set.user_to_group.contains_key("alice"));
    assert!(!group_set.user_to_group.contains_key("bob"));
}

#[test]
fn test_from_maps_missing_user_in_mapping() {
    // Test case 2: Group contains user not in user_to_group
    let user_to_group = HashMap::new(); // Empty mapping

    let mut groups = HashMap::new();
    let mut members = HashSet::new();
    members.insert("alice".to_string());
    members.insert("bob".to_string());
    groups.insert(
        0,
        GroupInfo {
            members,
            last_modified: OffsetDateTime::now_utc(),
        },
    );

    let group_set = GroupSets::from_maps(user_to_group, groups);

    // Both users should be added to user_to_group
    assert_eq!(group_set.user_to_group.len(), 2);
    assert_eq!(group_set.user_to_group.get("alice"), Some(&0));
    assert_eq!(group_set.user_to_group.get("bob"), Some(&0));
}

#[test]
fn test_from_maps_wrong_group_mapping() {
    // Test case 3: User points to wrong group in user_to_group
    let mut user_to_group = HashMap::new();
    user_to_group.insert("alice".to_string(), 1); // Points to wrong group

    let mut groups = HashMap::new();
    let mut members = HashSet::new();
    members.insert("alice".to_string());
    groups.insert(
        0,
        GroupInfo {
            members,
            last_modified: OffsetDateTime::now_utc(),
        },
    );

    let group_set = GroupSets::from_maps(user_to_group, groups);

    // Alice should be corrected to point to group 0
    assert_eq!(group_set.user_to_group.get("alice"), Some(&0));
}

#[test]
fn test_from_maps_empty_groups_removed() {
    // Test case 4: Empty groups should be removed
    let user_to_group = HashMap::new();

    let mut groups = HashMap::new();
    groups.insert(
        0,
        GroupInfo {
            members: HashSet::new(), // Empty group
            last_modified: OffsetDateTime::now_utc(),
        },
    );

    let mut valid_members = HashSet::new();
    valid_members.insert("alice".to_string());
    groups.insert(
        1,
        GroupInfo {
            members: valid_members,
            last_modified: OffsetDateTime::now_utc(),
        },
    );

    let group_set = GroupSets::from_maps(user_to_group, groups);

    // Empty group should be removed, alice should be in user_to_group
    assert_eq!(group_set.groups.len(), 1);
    assert!(!group_set.groups.contains_key(&0));
    assert!(group_set.groups.contains_key(&1));
    assert_eq!(group_set.user_to_group.get("alice"), Some(&1));
}

#[test]
fn test_from_maps_orphaned_users_removed() {
    // Test case 5: Users in user_to_group but not in any group should be removed
    let mut user_to_group = HashMap::new();
    user_to_group.insert("alice".to_string(), 0);
    user_to_group.insert("orphan".to_string(), 0);

    let mut groups = HashMap::new();
    let mut members = HashSet::new();
    members.insert("alice".to_string()); // orphan is not in the group
    groups.insert(
        0,
        GroupInfo {
            members,
            last_modified: OffsetDateTime::now_utc(),
        },
    );

    let group_set = GroupSets::from_maps(user_to_group, groups);

    // Orphan should be removed from user_to_group
    assert_eq!(group_set.user_to_group.len(), 1);
    assert!(group_set.user_to_group.contains_key("alice"));
    assert!(!group_set.user_to_group.contains_key("orphan"));
}

// ============================================================================
// FUZZ TESTS
// ============================================================================

fn check_invariants(group_set: &GroupSets) -> Result<(), String> {
    // Invariant 1: Every user in user_to_group must have a corresponding group
    for (user, group_id) in &group_set.user_to_group {
        if !group_set.groups.contains_key(group_id) {
            return Err(format!(
                "User {} points to non-existent group {}",
                user, group_id
            ));
        }
    }

    // Invariant 2: Every user in a group must be in user_to_group
    for (group_id, group_info) in &group_set.groups {
        for user in &group_info.members {
            match group_set.user_to_group.get(user) {
                Some(mapped_group_id) if mapped_group_id == group_id => {}
                Some(mapped_group_id) => {
                    return Err(format!(
                        "User {} in group {} but user_to_group maps to {}",
                        user, group_id, mapped_group_id
                    ));
                }
                None => {
                    return Err(format!(
                        "User {} in group {} but not in user_to_group",
                        user, group_id
                    ));
                }
            }
        }
    }

    // Invariant 3: No empty groups
    for (group_id, group_info) in &group_set.groups {
        if group_info.members.is_empty() {
            return Err(format!("Group {} is empty", group_id));
        }
    }

    // Invariant 4: next_group_id should be greater than all existing group IDs
    if let Some(max_group_id) = group_set.groups.keys().max().copied() {
        if group_set.next_group_id <= max_group_id {
            return Err(format!(
                "next_group_id {} should be > max group ID {}",
                group_set.next_group_id, max_group_id
            ));
        }
    }

    Ok(())
}

fn arbitrary_user() -> impl Strategy<Value = String> {
    prop::string::string_regex("[a-zA-Z][a-zA-Z0-9_]{0,15}").unwrap()
}

fn arbitrary_group_info() -> impl Strategy<Value = GroupInfo> {
    (
        prop::collection::hash_set(arbitrary_user(), 1..10),
        any::<i64>().prop_map(|secs| {
            OffsetDateTime::from_unix_timestamp(secs.abs() % 1_000_000_000)
                .unwrap_or_else(|_| OffsetDateTime::now_utc())
        }),
    )
        .prop_map(|(members, last_modified)| GroupInfo {
            members,
            last_modified,
        })
}

// Generate arbitrary but potentially inconsistent maps for from_maps testing
prop_compose! {
    fn arbitrary_maps()(
        user_to_group in prop::collection::hash_map(arbitrary_user(), any::<usize>(), 0..20),
        groups in prop::collection::hash_map(any::<usize>(), arbitrary_group_info(), 0..10)
    ) -> (HashMap<String, usize>, HashMap<usize, GroupInfo>) {
        (user_to_group, groups)
    }
}

#[derive(Debug, Clone)]
enum Operation {
    AddUser(String),
    Union(String, String),
    RemoveUser(String),
    GetGroupMembers(String),
    FindGroup(String),
    ExpireOldGroups(OffsetDateTime),
}

fn arbitrary_operation() -> impl Strategy<Value = Operation> {
    prop_oneof![
        arbitrary_user().prop_map(Operation::AddUser),
        (arbitrary_user(), arbitrary_user()).prop_map(|(u1, u2)| Operation::Union(u1, u2)),
        arbitrary_user().prop_map(Operation::RemoveUser),
        arbitrary_user().prop_map(Operation::GetGroupMembers),
        arbitrary_user().prop_map(Operation::FindGroup),
        any::<i64>().prop_map(|secs| {
            Operation::ExpireOldGroups(
                OffsetDateTime::from_unix_timestamp(secs.abs() % 1_000_000_000).unwrap(),
            )
        }),
    ]
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(1000))]

    /// Test that from_maps always produces a valid GroupSets regardless of input
    #[test]
    fn fuzz_from_maps_always_valid(
        (user_to_group, groups) in arbitrary_maps()
    ) {
        let group_set = GroupSets::from_maps(user_to_group, groups);
        prop_assert!(check_invariants(&group_set).is_ok(), "Invariants violated: {:?}", check_invariants(&group_set));
    }

    /// Test that from_maps preserves user relationships when input is consistent
    #[test]
    fn fuzz_from_maps_preserves_relationships(
        users in prop::collection::vec(arbitrary_user(), 1..10)
    ) {
        // Create a consistent state where all users are in one group
        let mut user_to_group = HashMap::new();
        let mut members = HashSet::new();

        for user in &users {
            user_to_group.insert(user.clone(), 42);
            members.insert(user.clone());
        }

        let mut groups = HashMap::new();
        groups.insert(42, GroupInfo {
            members,
            last_modified: OffsetDateTime::now_utc(),
        });

        let group_set = GroupSets::from_maps(user_to_group, groups);

        // All users should still be in the same group
        if users.len() > 1 {
            let first_user_group = group_set.find_group(&users[0]);
            for user in &users[1..] {
                prop_assert_eq!(group_set.find_group(user), first_user_group);
            }
        }
    }

    /// Test operation sequences maintain invariants
    #[test]
    fn fuzz_operation_sequences(
        operations in prop::collection::vec(arbitrary_operation(), 0..50)
    ) {
        let mut group_set = GroupSets::new();
        for op in operations {
            match &op {
                Operation::AddUser(user) =>
                    group_set.add_user(user),
                Operation::Union(user1, user2) =>
                    group_set.union(user1, user2),
                Operation::RemoveUser(user) =>
                    group_set.remove_user(user),
                Operation::GetGroupMembers(user) => {
                    let members = group_set.get_group_members(user);
                    if group_set.find_group(user).is_some() {
                        prop_assert!(members.contains(user), "User {} not in their own group members: {:?}", user, members);
                    }
                }
                Operation::FindGroup(user) => {
                    let _ = group_set.find_group(user);
                }
                Operation::ExpireOldGroups(now) =>
                    group_set.expire_old_groups(*now),
            }
            prop_assert!(check_invariants(&group_set).is_ok(), "Invariants violated after operation {:?}: {:?}", op, check_invariants(&group_set));
        }
    }

    /// Test union operation properties
    #[test]
    fn fuzz_union_properties(
        user1 in arbitrary_user(),
        user2 in arbitrary_user(),
        user3 in arbitrary_user()
    ) {
        let mut group_set = GroupSets::new();

        // Test reflexivity: union(a, a) should be safe
        group_set.union(&user1, &user1);
        prop_assert_eq!(group_set.find_group(&user1), group_set.find_group(&user1));

        // Test symmetry: union(a, b) == union(b, a)
        let mut group_set1 = GroupSets::new();
        let mut group_set2 = GroupSets::new();

        group_set1.union(&user1, &user2);
        group_set2.union(&user2, &user1);

        prop_assert_eq!(
            group_set1.find_group(&user1) == group_set1.find_group(&user2),
            group_set2.find_group(&user1) == group_set2.find_group(&user2)
        );

        // Test transitivity: if union(a,b) and union(b,c), then a and c should be connected
        let mut group_set = GroupSets::new();
        group_set.union(&user1, &user2);
        group_set.union(&user2, &user3);

        prop_assert_eq!(group_set.find_group(&user1), group_set.find_group(&user3));
    }

    /// Test that get_group_members is consistent with find_group
    #[test]
    fn fuzz_group_members_consistency(
        operations in prop::collection::vec(arbitrary_operation(), 0..30)
    ) {
        let mut group_set = GroupSets::new();

        // Apply operations
        for op in operations {
            match op {
                Operation::AddUser(user) => group_set.add_user(&user),
                Operation::Union(user1, user2) => group_set.union(&user1, &user2),
                Operation::RemoveUser(user) => group_set.remove_user(&user),
                Operation::ExpireOldGroups(now) => group_set.expire_old_groups(now),
                _ => {}
            }
        }

        // Check consistency for all users
        for user in group_set.user_to_group.keys() {
            let group_id = group_set.find_group(user);
            let members = group_set.get_group_members(user);

            // User should be in their own group members
            prop_assert!(members.contains(user), "User {} not in own group members", user);

            // All members should have the same group ID
            for member in &members {
                prop_assert_eq!(group_set.find_group(member), group_id,
                    "Member {} has different group ID than {}", member, user);
            }

            // Group members should match the actual group
            if let Some(gid) = group_id {
                if let Some(group_info) = group_set.groups.get(&gid) {
                    let mut expected_members: Vec<String> = group_info.members.iter().cloned().collect();
                    let mut actual_members = members.clone();
                    expected_members.sort();
                    actual_members.sort();
                    prop_assert_eq!(expected_members, actual_members);
                }
            }
        }
    }

    /// Test remove_user creates proper singleton
    #[test]
    fn fuzz_remove_user_creates_singleton(
        users in prop::collection::vec(arbitrary_user(), 2..10),
        remove_idx in any::<usize>()
    ) {
        let mut group_set = GroupSets::new();

        // Union all users into one group
        for i in 1..users.len() {
            group_set.union(&users[0], &users[i]);
        }

        // Verify they're all in the same group
        let original_group = group_set.find_group(&users[0]);
        for user in &users {
            prop_assert_eq!(group_set.find_group(user), original_group);
        }

        // Remove one user
        let remove_idx = remove_idx % users.len();
        let removed_user = &users[remove_idx];
        group_set.remove_user(removed_user);

        // Removed user should be in a different group (singleton)
        prop_assert_ne!(group_set.find_group(removed_user), original_group);
        prop_assert_eq!(group_set.get_group_members(removed_user), vec![removed_user.clone()]);

        // Other users should still be together (if more than one remains)
        let remaining_users = users.iter().filter(|u| *u != removed_user).collect::<Vec<_>>();
        if remaining_users.len() > 1 {
            let first_remaining_group = group_set.find_group(remaining_users[0]);
            for user in &remaining_users[1..] {
                prop_assert_eq!(group_set.find_group(user), first_remaining_group);
            }
        }
    }
}
