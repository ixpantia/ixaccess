# Plan: Role Deletion via State Rebalancing

This plan outlines the implementation of role deletion in `ixaccess` without changing the underlying file format version. Deletion will be achieved by reconstructing the state (rebalancing) to ensure the internal interners and graphs remain compact and consistent.

## 1. Core Logic (Rust Internals)

Modify `ixaccess/rs/ixaccess/src/internals/structure/roles.rs` to add:

- **`delete_role(&mut self, role_to_delete: &Role)`**:
    - **Step 1: Check Existence**: Look up the `RoleId` of the target role. If it doesn't exist, return early.
    - **Step 2: Initialize New State**: Create a fresh `IxAccessStructureV1`.
    - **Step 3: Transfer Resources**: Move the `resource_resolver` interner to the new state (resource tags/values don't change).
    - **Step 4: Re-intern Roles**: Iterate through the current `role_resolver`. Intern all roles *except* the deleted one into the new state's `role_resolver`.
    - **Step 5: Re-map Inheritance**: 
        - Iterate through the old `role_graph`.
        - For each role $R_{old}$ that is not the deleted role:
            - Find its corresponding new ID $R_{new}$.
            - For each child role $C_{old}$ in its graph entry:
                - If $C_{old}$ is not the deleted role, find $C_{new}$ and add it to $R_{new}$'s entry in the new graph.
    - **Step 6: Re-map Resource Assignments**:
        - Iterate through the old `resource_assignment` vector.
        - Copy assignments for all non-deleted roles to their new corresponding indices in the new state.
    - **Step 7: Swap State**: Replace `*self` with the newly constructed structure.

## 2. Client API Updates

- **Rust (`ixaccess/rs/ixaccess/src/client/roles.rs`)**:
    - Add `pub async fn delete_role(&self, role: impl AsRef<str>) -> Result<(), StorageError>`.
    - Use `self.storage.update_file` to perform the download-modify-upload cycle atomically.

- **Python (`ixaccess/py/ixaccess/src/lib.rs`)**:
    - Add `fn delete_role(&self, role: &str) -> PyResult<()>` to the `IxAccessClient` implementation.

- **R (`ixaccess/r/ixaccess/src/rust/src/lib.rs`)**:
    - Add `fn delete_role(&self, role: &str) -> Result<()>` to `IxAccessClientInternal`.

## 3. Verification Plan

- **Unit Tests**: Add tests in `ixaccess/rs/ixaccess/src/internals/structure/tests.rs`:
    - Delete a leaf role.
    - Delete a parent role (ensure children no longer inherit from it).
    - Delete a role with resources (ensure resources are purged).
    - Verify `list_roles()` no longer returns the deleted role.
    - Verify that re-adding a deleted role works correctly (starting from a clean state).

## 4. Advantages

- **Backward Compatibility**: No change to `Version 1` schema.
- **Forward Compatibility**: Older versions of the library can read files where roles were deleted by newer versions.
- **Efficiency**: While $O(N)$ on the number of roles, it keeps the storage file compact by removing unused strings and graph nodes.