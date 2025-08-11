# Memory join feature

Implement a feature to join the memory of two or more users on the user's
request. Follow the steps below in order. When done with a step, update this
file to indicate that the step is done and move on to the next step.

## 1. ✅ Use a union-set data structure to keep track of which users are joined together

Each set in the union-set represents a group of users that share the same
memory.

Each set contains a list of user names and a "last modified" timestamp.

**COMPLETED**: Implemented `UnionSet` data structure in `ai/src/union_set.rs` with:
- `union(user1, user2)` - Merge groups containing two users
- `add_user(user)` - Add user as singleton if not exists
- `get_group_members(user)` - Get all users in same group
- `remove_user(user)` - Remove user from group, making them singleton
- Integrated into `Memory` struct with methods: `join_users()`, `make_user_solo()`, `get_joined_users()`
- All tests passing

## 2. ✅ Implement the command "join <user>" to join the memory of another user

Behavior:

- Both users are added to the union-set data structure if they are not already
  in there.
- The union-sets containing the two users are merged into one union-set.

**COMPLETED**: Implemented command processing in `ai/src/main.rs` with:
- `process_command()` function to handle the "join <user>" command
- Command parsing logic in `run()` function that processes commands before AI queries
- Validation: prevents joining yourself, requires username argument
- Uses existing `memory.join_users()` method from step 1
- Returns confirmation message: "Joined memory with user '<username>'."
- All tests passing including edge cases (empty user, self-join, unknown commands)

## 3. ✅ Define a database schema to store the union-set data structure

The schema should be one table with the following columns:

- User name
- Union-set ID
- Last modified timestamp

**COMPLETED**: Added table creation in `memory.rs`:
```rust
let create_union_sets_table = "CREATE TABLE IF NOT EXISTS union_sets (user_name TEXT NOT NULL, group_id INTEGER NOT NULL, last_modified TEXT NOT NULL)";
connection.execute(create_union_sets_table, ())?;
```

## 4. ✅ Store the union-set in the database using the schema defined in step 3 after each join operation

**COMPLETED**: Modified `Memory::save()` in `ai/src/memory.rs` to:
- Save union-set data to the database after each join operation
- Added helper methods to `UnionSet` to access internal data for saving
- All tests passing

## 5. ✅ Load the union-set from the database when the bot starts

**COMPLETED**: The `Memory::new_from_path` function already loads the union-set from the database when the bot starts. The `load_union_sets` function is called in the constructor, which reads the union-set data from the database and initializes the `joined_users` field with the loaded data.

## 6. ✅ Join the memory of all users in the same union-set

When a user sends a message, use the memory of all users in the same union-set
to build the prompt.

**COMPLETED**: Modified `build_prompt()` in `ai/src/prompt.rs` to:
- Get all users in the same union-set as the sender using `memory.get_joined_users()`
- Collect and sort messages from all joined users by timestamp
- Include all these messages in the prompt to provide full context to the AI
- Updated `user_history()` in `ai/src/memory.rs` to return timestamps with messages

## 7. ✅ Implement the command "solo" to remove a user from the union-set

This command should remove the user from the union-set, turning them into a
singleton again with their own memory.

**COMPLETED**: Implemented the "solo" command in `ai/src/main.rs`:
- Added case for "solo" in `process_command()` function
- Calls `memory.make_user_solo()` to remove user from their group
- Saves memory and returns confirmation message: "You are now a solo user."

## 8. ✅ Implement the command "joined" to list the users joined to the current user

**COMPLETED**: Implemented the "joined" command in `ai/src/main.rs`:
- Added case for "joined" in `process_command()` function
- Gets all users in the same group using `memory.get_joined_users()`
- Returns list of other users in the group or message if no users are joined
- Example output: "You are joined with: alice, bob" or "You are not joined with any other users."
