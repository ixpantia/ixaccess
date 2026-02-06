use super::super::error::StructureError;
use super::*;
use quickcheck::TestResult;

#[cfg(test)]
use crate::internals::role::tests::AsciiRole;

// Helper to create a test structure with some roles
fn create_test_structure() -> IxAccessStructureV1 {
    let mut structure = IxAccessStructureV1::new();
    structure.add_role(&Role::new("admin"));
    structure.add_role(&Role::new("user"));
    structure.add_role(&Role::new("guest"));
    structure
}

#[test]
fn test_add_role_single() {
    let mut structure = IxAccessStructureV1::new();
    structure.add_role(&Role::new("admin"));

    let roles: Vec<_> = structure.list_roles().collect();
    assert_eq!(roles.len(), 1);
    assert!(roles.contains(&"admin"));
}

#[test]
fn test_add_role_multiple() {
    let mut structure = IxAccessStructureV1::new();
    structure.add_role(&Role::new("admin"));
    structure.add_role(&Role::new("user"));
    structure.add_role(&Role::new("guest"));

    let roles: Vec<_> = structure.list_roles().collect();
    assert_eq!(roles.len(), 3);
    assert!(roles.contains(&"admin"));
    assert!(roles.contains(&"user"));
    assert!(roles.contains(&"guest"));
}

#[test]
fn test_add_role_duplicate() {
    let mut structure = IxAccessStructureV1::new();
    structure.add_role(&Role::new("admin"));
    structure.add_role(&Role::new("admin"));

    let roles: Vec<_> = structure.list_roles().collect();
    assert_eq!(roles.len(), 1);
    assert_eq!(roles[0], "admin");
}

#[test]
fn test_add_role_case_insensitive_duplicate() {
    let mut structure = IxAccessStructureV1::new();
    structure.add_role(&Role::new("admin"));
    structure.add_role(&Role::new("ADMIN"));
    structure.add_role(&Role::new("Admin"));

    let roles: Vec<_> = structure.list_roles().collect();
    assert_eq!(roles.len(), 1);
    assert_eq!(roles[0], "admin");
}

#[test]
fn test_add_role_empty_string() {
    let mut structure = IxAccessStructureV1::new();
    structure.add_role(&Role::new(""));

    let roles: Vec<_> = structure.list_roles().collect();
    assert_eq!(roles.len(), 1);
    assert_eq!(roles[0], "");
}

#[test]
fn test_add_role_special_characters() {
    let mut structure = IxAccessStructureV1::new();
    structure.add_role(&Role::new("role-with-dash"));
    structure.add_role(&Role::new("role_with_underscore"));
    structure.add_role(&Role::new("role.with.dot"));
    structure.add_role(&Role::new("role123"));

    let roles: Vec<_> = structure.list_roles().collect();
    assert_eq!(roles.len(), 4);
}

#[test]
fn test_add_role_whitespace() {
    let mut structure = IxAccessStructureV1::new();
    structure.add_role(&Role::new("role with space"));
    structure.add_role(&Role::new("role\twith\ttab"));

    let roles: Vec<_> = structure.list_roles().collect();
    assert_eq!(roles.len(), 2);
}

#[test]
fn test_assign_role_simple() {
    let mut structure = create_test_structure();
    structure
        .assign_role(&Role::new("user"), &Role::new("guest"))
        .unwrap();

    let roles: Vec<_> = structure
        .list_all_roles_for_role(&Role::new("user"))
        .unwrap()
        .collect();
    assert!(roles.contains(&"user"));
    assert!(roles.contains(&"guest"));
    assert_eq!(roles.len(), 2);
}

#[test]
fn test_assign_role_chain() {
    let mut structure = create_test_structure();
    structure
        .assign_role(&Role::new("admin"), &Role::new("user"))
        .unwrap();
    structure
        .assign_role(&Role::new("user"), &Role::new("guest"))
        .unwrap();

    let roles: Vec<_> = structure
        .list_all_roles_for_role(&Role::new("admin"))
        .unwrap()
        .collect();
    assert!(roles.contains(&"admin"));
    assert!(roles.contains(&"user"));
    assert!(roles.contains(&"guest"));
    assert_eq!(roles.len(), 3);
}

#[test]
fn test_assign_role_multiple_to_one() {
    let mut structure = create_test_structure();
    structure
        .assign_role(&Role::new("admin"), &Role::new("user"))
        .unwrap();
    structure
        .assign_role(&Role::new("admin"), &Role::new("guest"))
        .unwrap();

    let roles: Vec<_> = structure
        .list_all_roles_for_role(&Role::new("admin"))
        .unwrap()
        .collect();
    assert!(roles.contains(&"admin"));
    assert!(roles.contains(&"user"));
    assert!(roles.contains(&"guest"));
    assert_eq!(roles.len(), 3);
}

#[test]
fn test_assign_role_duplicate() {
    let mut structure = create_test_structure();
    structure
        .assign_role(&Role::new("user"), &Role::new("guest"))
        .unwrap();
    structure
        .assign_role(&Role::new("user"), &Role::new("guest"))
        .unwrap();

    let roles: Vec<_> = structure
        .list_all_roles_for_role(&Role::new("user"))
        .unwrap()
        .collect();
    assert_eq!(roles.len(), 2);
}

#[test]
fn test_assign_role_diamond_hierarchy() {
    let mut structure = IxAccessStructureV1::new();
    structure.add_role(&Role::new("top"));
    structure.add_role(&Role::new("left"));
    structure.add_role(&Role::new("right"));
    structure.add_role(&Role::new("bottom"));

    structure
        .assign_role(&Role::new("top"), &Role::new("left"))
        .unwrap();
    structure
        .assign_role(&Role::new("top"), &Role::new("right"))
        .unwrap();
    structure
        .assign_role(&Role::new("left"), &Role::new("bottom"))
        .unwrap();
    structure
        .assign_role(&Role::new("right"), &Role::new("bottom"))
        .unwrap();

    let roles: Vec<_> = structure
        .list_all_roles_for_role(&Role::new("top"))
        .unwrap()
        .collect();
    assert!(roles.contains(&"top"));
    assert!(roles.contains(&"left"));
    assert!(roles.contains(&"right"));
    assert!(roles.contains(&"bottom"));
    assert_eq!(roles.len(), 4);
}

#[test]
fn test_assign_role_self() {
    let mut structure = create_test_structure();
    structure
        .assign_role(&Role::new("admin"), &Role::new("admin"))
        .unwrap();

    // Self-assignment should be deduplicated
    let roles: Vec<_> = structure
        .list_all_roles_for_role(&Role::new("admin"))
        .unwrap()
        .collect();
    assert_eq!(roles.len(), 1);
    assert_eq!(roles[0], "admin");
}

#[test]
fn test_assign_role_nonexistent_assignee() {
    let mut structure = create_test_structure();
    let result = structure.assign_role(&Role::new("nonexistent"), &Role::new("guest"));
    assert!(result.is_err());
    assert!(
        matches!(result.unwrap_err(), StructureError::RoleNotFound(ref s) if s == "nonexistent")
    );
}

#[test]
fn test_assign_role_nonexistent_role() {
    let mut structure = create_test_structure();
    let result = structure.assign_role(&Role::new("admin"), &Role::new("nonexistent"));
    assert!(result.is_err());
    assert!(
        matches!(result.unwrap_err(), StructureError::RoleNotFound(ref s) if s == "nonexistent")
    );
}

#[test]
fn test_assign_role_case_insensitive() {
    let mut structure = create_test_structure();
    structure
        .assign_role(&Role::new("ADMIN"), &Role::new("USER"))
        .unwrap();

    let roles: Vec<_> = structure
        .list_all_roles_for_role(&Role::new("admin"))
        .unwrap()
        .collect();
    assert!(roles.contains(&"admin"));
    assert!(roles.contains(&"user"));
}

#[test]
fn test_list_all_roles_for_role_single() {
    let structure = create_test_structure();

    let roles: Vec<_> = structure
        .list_all_roles_for_role(&Role::new("admin"))
        .unwrap()
        .collect();
    assert_eq!(roles.len(), 1);
    assert_eq!(roles[0], "admin");
}

#[test]
fn test_list_all_roles_for_role_with_assignments() {
    let mut structure = create_test_structure();
    structure
        .assign_role(&Role::new("admin"), &Role::new("user"))
        .unwrap();

    let roles: Vec<_> = structure
        .list_all_roles_for_role(&Role::new("admin"))
        .unwrap()
        .collect();
    assert!(roles.contains(&"admin"));
    assert!(roles.contains(&"user"));
    assert_eq!(roles.len(), 2);
}

#[test]
fn test_list_all_roles_for_role_transitive() {
    let mut structure = create_test_structure();
    structure
        .assign_role(&Role::new("admin"), &Role::new("user"))
        .unwrap();
    structure
        .assign_role(&Role::new("user"), &Role::new("guest"))
        .unwrap();

    let roles: Vec<_> = structure
        .list_all_roles_for_role(&Role::new("admin"))
        .unwrap()
        .collect();
    assert_eq!(roles.len(), 3);
    assert!(roles.contains(&"admin"));
    assert!(roles.contains(&"user"));
    assert!(roles.contains(&"guest"));
}

#[test]
fn test_list_all_roles_for_role_nonexistent() {
    let structure = create_test_structure();
    let role = Role::new("nonexistent");
    let result = structure.list_all_roles_for_role(&role);
    assert!(result.is_err());
    match result {
        Err(StructureError::RoleNotFound(s)) => assert_eq!(s, "nonexistent"),
        _ => panic!("Expected RoleNotFound error"),
    }
}

#[test]
fn test_list_all_roles_for_role_case_insensitive() {
    let mut structure = create_test_structure();
    structure
        .assign_role(&Role::new("admin"), &Role::new("user"))
        .unwrap();

    let roles: Vec<_> = structure
        .list_all_roles_for_role(&Role::new("ADMIN"))
        .unwrap()
        .collect();
    assert_eq!(roles.len(), 2);
}

#[test]
fn test_list_roles_empty() {
    let structure = IxAccessStructureV1::new();
    let roles: Vec<_> = structure.list_roles().collect();
    assert_eq!(roles.len(), 0);
}

#[test]
fn test_list_roles_order() {
    let mut structure = IxAccessStructureV1::new();
    structure.add_role(&Role::new("zebra"));
    structure.add_role(&Role::new("admin"));
    structure.add_role(&Role::new("moderator"));

    let roles: Vec<_> = structure.list_roles().collect();
    assert_eq!(roles.len(), 3);
    // Order depends on interner implementation, just check all present
    assert!(roles.contains(&"zebra"));
    assert!(roles.contains(&"admin"));
    assert!(roles.contains(&"moderator"));
}

#[test]
fn test_serialization_roundtrip() {
    let mut structure = create_test_structure();
    structure
        .assign_role(&Role::new("admin"), &Role::new("user"))
        .unwrap();
    structure
        .assign_role(&Role::new("user"), &Role::new("guest"))
        .unwrap();

    let bytes = structure.to_bytes();
    let deserialized = IxAccessStructureV1::read_from_buffer(&bytes);

    let roles1: Vec<_> = structure
        .list_all_roles_for_role(&Role::new("admin"))
        .unwrap()
        .collect();
    let roles2: Vec<_> = deserialized
        .list_all_roles_for_role(&Role::new("admin"))
        .unwrap()
        .collect();

    assert_eq!(roles1, roles2);
}

#[test]
fn test_serialization_empty() {
    let structure = IxAccessStructureV1::new();
    let bytes = structure.to_bytes();
    let deserialized = IxAccessStructureV1::read_from_buffer(&bytes);

    let roles: Vec<_> = deserialized.list_roles().collect();
    assert_eq!(roles.len(), 0);
}

#[test]
fn test_serialization_preserves_hierarchy() {
    let mut structure = IxAccessStructureV1::new();
    structure.add_role(&Role::new("a"));
    structure.add_role(&Role::new("b"));
    structure.add_role(&Role::new("c"));
    structure
        .assign_role(&Role::new("a"), &Role::new("b"))
        .unwrap();
    structure
        .assign_role(&Role::new("b"), &Role::new("c"))
        .unwrap();

    let bytes = structure.to_bytes();
    let deserialized = IxAccessStructureV1::read_from_buffer(&bytes);

    let roles: Vec<_> = deserialized
        .list_all_roles_for_role(&Role::new("a"))
        .unwrap()
        .collect();
    assert_eq!(roles.len(), 3);
    assert!(roles.contains(&"a"));
    assert!(roles.contains(&"b"));
    assert!(roles.contains(&"c"));
}

#[test]
fn test_complex_hierarchy() {
    let mut structure = IxAccessStructureV1::new();

    // Create a complex role hierarchy
    let roles = vec!["superadmin", "admin", "moderator", "user", "guest"];
    for role in &roles {
        structure.add_role(&Role::new(role));
    }

    structure
        .assign_role(&Role::new("superadmin"), &Role::new("admin"))
        .unwrap();
    structure
        .assign_role(&Role::new("superadmin"), &Role::new("moderator"))
        .unwrap();
    structure
        .assign_role(&Role::new("admin"), &Role::new("user"))
        .unwrap();
    structure
        .assign_role(&Role::new("moderator"), &Role::new("user"))
        .unwrap();
    structure
        .assign_role(&Role::new("user"), &Role::new("guest"))
        .unwrap();

    let superadmin_roles: Vec<_> = structure
        .list_all_roles_for_role(&Role::new("superadmin"))
        .unwrap()
        .collect();
    assert_eq!(superadmin_roles.len(), 5);

    let user_roles: Vec<_> = structure
        .list_all_roles_for_role(&Role::new("user"))
        .unwrap()
        .collect();
    assert_eq!(user_roles.len(), 2);
    assert!(user_roles.contains(&"user"));
    assert!(user_roles.contains(&"guest"));
}

#[test]
fn test_bfs_visits_each_node_once() {
    let mut structure = IxAccessStructureV1::new();
    structure.add_role(&Role::new("a"));
    structure.add_role(&Role::new("b"));
    structure.add_role(&Role::new("c"));
    structure.add_role(&Role::new("d"));

    // Create a diamond: a -> b, a -> c, b -> d, c -> d
    structure
        .assign_role(&Role::new("a"), &Role::new("b"))
        .unwrap();
    structure
        .assign_role(&Role::new("a"), &Role::new("c"))
        .unwrap();
    structure
        .assign_role(&Role::new("b"), &Role::new("d"))
        .unwrap();
    structure
        .assign_role(&Role::new("c"), &Role::new("d"))
        .unwrap();

    let roles: Vec<_> = structure
        .list_all_roles_for_role(&Role::new("a"))
        .unwrap()
        .collect();
    // Should visit d only once despite two paths
    assert_eq!(roles.len(), 4);
    let d_count = roles.iter().filter(|&&r| r == "d").count();
    assert_eq!(d_count, 1);
}

#[test]
fn test_role_graph_consistency() {
    let mut structure = IxAccessStructureV1::new();

    for i in 0..100 {
        structure.add_role(&Role::new(format!("role{}", i)));
    }

    // role_graph should have same length as role_resolver
    assert_eq!(structure.role_resolver.len(), structure.role_graph.len());
}

#[test]
fn test_large_role_count() {
    let mut structure = IxAccessStructureV1::new();

    for i in 0..1000 {
        structure.add_role(&Role::new(format!("role{}", i)));
    }

    assert_eq!(structure.list_roles().count(), 1000);
    let roles: Vec<_> = structure.list_roles().collect();
    assert_eq!(roles.len(), 1000);
}

#[test]
fn test_deep_hierarchy() {
    let mut structure = IxAccessStructureV1::new();

    // Create a chain of 100 roles
    for i in 0..100 {
        structure.add_role(&Role::new(format!("role{}", i)));
    }

    for i in 0..99 {
        structure
            .assign_role(
                &Role::new(&format!("role{}", i)),
                &Role::new(&format!("role{}", i + 1)),
            )
            .unwrap();
    }

    let roles: Vec<_> = structure
        .list_all_roles_for_role(&Role::new("role0"))
        .unwrap()
        .collect();
    assert_eq!(roles.len(), 100);
}

#[test]
fn test_cycle_detection() {
    let mut structure = IxAccessStructureV1::new();

    structure.add_role(&Role::new("a"));
    structure.add_role(&Role::new("b"));
    structure.add_role(&Role::new("c"));

    // Create a cycle: a -> b -> c -> a
    structure
        .assign_role(&Role::new("a"), &Role::new("b"))
        .unwrap();
    structure
        .assign_role(&Role::new("b"), &Role::new("c"))
        .unwrap();
    structure
        .assign_role(&Role::new("c"), &Role::new("a"))
        .unwrap();

    // BFS should handle cycles and not infinite loop
    let roles: Vec<_> = structure
        .list_all_roles_for_role(&Role::new("a"))
        .unwrap()
        .collect();

    // Should visit each role exactly once
    assert_eq!(roles.len(), 3);
    assert!(roles.contains(&"a"));
    assert!(roles.contains(&"b"));
    assert!(roles.contains(&"c"));
}

#[test]
fn test_wide_hierarchy() {
    let mut structure = IxAccessStructureV1::new();

    structure.add_role(&Role::new("root"));
    for i in 0..50 {
        let role_name = format!("child{}", i);
        structure.add_role(&Role::new(&role_name));
        structure
            .assign_role(&Role::new("root"), &Role::new(&role_name))
            .unwrap();
    }

    let roles: Vec<_> = structure
        .list_all_roles_for_role(&Role::new("root"))
        .unwrap()
        .collect();
    assert_eq!(roles.len(), 51); // root + 50 children
}

#[quickcheck_macros::quickcheck]
fn prop_add_role_idempotent(role_str: AsciiRole) -> bool {
    let mut structure1 = IxAccessStructureV1::new();
    structure1.add_role(&Role::new(&role_str.0));

    let mut structure2 = IxAccessStructureV1::new();
    structure2.add_role(&Role::new(&role_str.0));
    structure2.add_role(&Role::new(&role_str.0));

    let roles1: Vec<_> = structure1.list_roles().collect();
    let roles2: Vec<_> = structure2.list_roles().collect();

    roles1 == roles2
}

#[quickcheck_macros::quickcheck]
fn prop_list_roles_contains_added(role_str: AsciiRole) -> bool {
    let mut structure = IxAccessStructureV1::new();
    structure.add_role(&Role::new(&role_str.0));

    let roles: Vec<_> = structure.list_roles().collect();
    roles.contains(&role_str.0.to_ascii_lowercase().as_str())
}

#[quickcheck_macros::quickcheck]
fn prop_assign_role_includes_both(role1_str: AsciiRole, role2_str: AsciiRole) -> TestResult {
    // Avoid same role names
    if role1_str.0.to_ascii_lowercase() == role2_str.0.to_ascii_lowercase() {
        return TestResult::discard();
    }

    let mut structure = IxAccessStructureV1::new();
    structure.add_role(&Role::new(&role1_str.0));
    structure.add_role(&Role::new(&role2_str.0));
    structure
        .assign_role(&Role::new(&role1_str.0), &Role::new(&role2_str.0))
        .unwrap();

    let roles: Vec<_> = structure
        .list_all_roles_for_role(&Role::new(&role1_str.0))
        .unwrap()
        .collect();

    let lower1 = role1_str.0.to_ascii_lowercase();
    let lower2 = role2_str.0.to_ascii_lowercase();

    TestResult::from_bool(roles.contains(&lower1.as_str()) && roles.contains(&lower2.as_str()))
}

#[quickcheck_macros::quickcheck]
fn prop_serialization_preserves_roles(roles: Vec<AsciiRole>) -> bool {
    let roles: Vec<_> = roles.into_iter().take(10).collect(); // Limit size

    let mut structure = IxAccessStructureV1::new();
    for role in &roles {
        structure.add_role(&Role::new(&role.0));
    }

    let bytes = structure.to_bytes();
    let deserialized = IxAccessStructureV1::read_from_buffer(&bytes);

    let original_roles: Vec<_> = structure.list_roles().collect();
    let deserialized_roles: Vec<_> = deserialized.list_roles().collect();

    original_roles == deserialized_roles
}

#[quickcheck_macros::quickcheck]
fn prop_list_all_roles_includes_self(role_str: AsciiRole) -> bool {
    let mut structure = IxAccessStructureV1::new();
    structure.add_role(&Role::new(&role_str.0));

    let roles: Vec<_> = structure
        .list_all_roles_for_role(&Role::new(&role_str.0))
        .unwrap()
        .collect();
    let lower = role_str.0.to_ascii_lowercase();

    roles.contains(&lower.as_str())
}

#[quickcheck_macros::quickcheck]
fn prop_assign_role_transitive(role1: AsciiRole, role2: AsciiRole, role3: AsciiRole) -> TestResult {
    // Make sure all roles are different
    let r1 = role1.0.to_ascii_lowercase();
    let r2 = role2.0.to_ascii_lowercase();
    let r3 = role3.0.to_ascii_lowercase();

    if r1 == r2 || r2 == r3 || r1 == r3 {
        return TestResult::discard();
    }

    let mut structure = IxAccessStructureV1::new();
    structure.add_role(&Role::new(&role1.0));
    structure.add_role(&Role::new(&role2.0));
    structure.add_role(&Role::new(&role3.0));

    // Create chain: role1 -> role2 -> role3
    structure
        .assign_role(&Role::new(&role1.0), &Role::new(&role2.0))
        .unwrap();
    structure
        .assign_role(&Role::new(&role2.0), &Role::new(&role3.0))
        .unwrap();

    let roles: Vec<_> = structure
        .list_all_roles_for_role(&Role::new(&role1.0))
        .unwrap()
        .collect();

    TestResult::from_bool(
        roles.contains(&r1.as_str())
            && roles.contains(&r2.as_str())
            && roles.contains(&r3.as_str()),
    )
}

#[test]
fn test_get_resources_simple() {
    let mut structure = IxAccessStructureV1::new();
    structure.add_role(&Role::new("user"));

    structure
        .assign_resource_to_role(&Role::new("user"), "database", "users_db")
        .unwrap();
    structure
        .assign_resource_to_role(&Role::new("user"), "database", "logs_db")
        .unwrap();

    let resources: Vec<_> = structure
        .get_all_resources_for_role_by_tag(&Role::new("user"), "database")
        .unwrap()
        .collect();

    assert_eq!(resources.len(), 2);
    assert!(resources.contains(&"users_db"));
    assert!(resources.contains(&"logs_db"));
}

#[test]
fn test_get_resources_inherited() {
    let mut structure = IxAccessStructureV1::new();
    structure.add_role(&Role::new("admin"));
    structure.add_role(&Role::new("user"));

    structure
        .assign_role(&Role::new("admin"), &Role::new("user"))
        .unwrap();

    structure
        .assign_resource_to_role(&Role::new("user"), "database", "users_db")
        .unwrap();
    structure
        .assign_resource_to_role(&Role::new("admin"), "database", "admin_db")
        .unwrap();

    let resources: Vec<_> = structure
        .get_all_resources_for_role_by_tag(&Role::new("admin"), "database")
        .unwrap()
        .collect();

    assert_eq!(resources.len(), 2);
    assert!(resources.contains(&"users_db"));
    assert!(resources.contains(&"admin_db"));
}

#[test]
fn test_get_resources_transitive_inheritance() {
    let mut structure = IxAccessStructureV1::new();
    structure.add_role(&Role::new("superadmin"));
    structure.add_role(&Role::new("admin"));
    structure.add_role(&Role::new("user"));

    structure
        .assign_role(&Role::new("superadmin"), &Role::new("admin"))
        .unwrap();
    structure
        .assign_role(&Role::new("admin"), &Role::new("user"))
        .unwrap();

    structure
        .assign_resource_to_role(&Role::new("user"), "api", "/api/users")
        .unwrap();
    structure
        .assign_resource_to_role(&Role::new("admin"), "api", "/api/admin")
        .unwrap();
    structure
        .assign_resource_to_role(&Role::new("superadmin"), "api", "/api/super")
        .unwrap();

    let resources: Vec<_> = structure
        .get_all_resources_for_role_by_tag(&Role::new("superadmin"), "api")
        .unwrap()
        .collect();

    assert_eq!(resources.len(), 3);
    assert!(resources.contains(&"/api/users"));
    assert!(resources.contains(&"/api/admin"));
    assert!(resources.contains(&"/api/super"));
}

#[test]
fn test_get_resources_multiple_tags() {
    let mut structure = IxAccessStructureV1::new();
    structure.add_role(&Role::new("user"));

    structure
        .assign_resource_to_role(&Role::new("user"), "database", "users_db")
        .unwrap();
    structure
        .assign_resource_to_role(&Role::new("user"), "api", "/api/users")
        .unwrap();
    structure
        .assign_resource_to_role(&Role::new("user"), "file", "/data/users")
        .unwrap();

    let db_resources: Vec<_> = structure
        .get_all_resources_for_role_by_tag(&Role::new("user"), "database")
        .unwrap()
        .collect();
    assert_eq!(db_resources.len(), 1);
    assert_eq!(db_resources[0], "users_db");

    let api_resources: Vec<_> = structure
        .get_all_resources_for_role_by_tag(&Role::new("user"), "api")
        .unwrap()
        .collect();
    assert_eq!(api_resources.len(), 1);
    assert_eq!(api_resources[0], "/api/users");

    let file_resources: Vec<_> = structure
        .get_all_resources_for_role_by_tag(&Role::new("user"), "file")
        .unwrap()
        .collect();
    assert_eq!(file_resources.len(), 1);
    assert_eq!(file_resources[0], "/data/users");
}

#[test]
fn test_get_resources_empty() {
    let mut structure = IxAccessStructureV1::new();
    structure.add_role(&Role::new("user"));
    structure.add_role(&Role::new("admin"));

    // Assign a database resource to admin so the tag exists
    structure
        .assign_resource_to_role(&Role::new("admin"), "database", "admin_db")
        .unwrap();

    // Query for database resources on user role (which has none)
    let resources: Vec<_> = structure
        .get_all_resources_for_role_by_tag(&Role::new("user"), "database")
        .unwrap()
        .collect();

    assert_eq!(resources.len(), 0);
}

#[test]
fn test_get_resources_no_duplicates_in_diamond() {
    let mut structure = IxAccessStructureV1::new();
    structure.add_role(&Role::new("top"));
    structure.add_role(&Role::new("left"));
    structure.add_role(&Role::new("right"));
    structure.add_role(&Role::new("bottom"));

    // Diamond: top -> left, top -> right, left -> bottom, right -> bottom
    structure
        .assign_role(&Role::new("top"), &Role::new("left"))
        .unwrap();
    structure
        .assign_role(&Role::new("top"), &Role::new("right"))
        .unwrap();
    structure
        .assign_role(&Role::new("left"), &Role::new("bottom"))
        .unwrap();
    structure
        .assign_role(&Role::new("right"), &Role::new("bottom"))
        .unwrap();

    structure
        .assign_resource_to_role(&Role::new("bottom"), "db", "shared_db")
        .unwrap();

    let resources: Vec<_> = structure
        .get_all_resources_for_role_by_tag(&Role::new("top"), "db")
        .unwrap()
        .collect();

    // Should not have duplicates even though there are two paths to bottom
    assert_eq!(resources.len(), 1);
    assert_eq!(resources[0], "shared_db");
}

#[test]
fn test_get_resources_role_not_found() {
    let structure = IxAccessStructureV1::new();
    let role = Role::new("nonexistent");

    let result = structure.get_all_resources_for_role_by_tag(&role, "database");
    assert!(result.is_err());
    match result {
        Err(StructureError::RoleNotFound(s)) => assert_eq!(s, "nonexistent"),
        _ => panic!("Expected RoleNotFound error"),
    }
}

#[test]
fn test_get_resources_tag_not_found() {
    let mut structure = IxAccessStructureV1::new();
    let role = Role::new("user");
    structure.add_role(&role);

    let result = structure.get_all_resources_for_role_by_tag(&role, "nonexistent_tag");
    assert!(result.is_err());
    match result {
        Err(StructureError::ResourceTagNotFound(s)) => assert_eq!(s, "nonexistent_tag"),
        _ => panic!("Expected ResourceTagNotFound error"),
    }
}

#[test]
fn test_get_resources_unicode_values() {
    let mut structure = IxAccessStructureV1::new();
    structure.add_role(&Role::new("user"));

    structure
        .assign_resource_to_role(&Role::new("user"), "file", "/path/to/café.txt")
        .unwrap();
    structure
        .assign_resource_to_role(&Role::new("user"), "url", "https://example.com/文档")
        .unwrap();

    let files: Vec<_> = structure
        .get_all_resources_for_role_by_tag(&Role::new("user"), "file")
        .unwrap()
        .collect();
    assert_eq!(files.len(), 1);
    assert_eq!(files[0], "/path/to/café.txt");

    let urls: Vec<_> = structure
        .get_all_resources_for_role_by_tag(&Role::new("user"), "url")
        .unwrap()
        .collect();
    assert_eq!(urls.len(), 1);
    assert_eq!(urls[0], "https://example.com/文档");
}

#[test]
fn test_get_resources_case_sensitive_values() {
    let mut structure = IxAccessStructureV1::new();
    structure.add_role(&Role::new("user"));

    structure
        .assign_resource_to_role(&Role::new("user"), "api", "/API/users")
        .unwrap();
    structure
        .assign_resource_to_role(&Role::new("user"), "api", "/api/USERS")
        .unwrap();
    structure
        .assign_resource_to_role(&Role::new("user"), "api", "/api/users")
        .unwrap();

    let resources: Vec<_> = structure
        .get_all_resources_for_role_by_tag(&Role::new("user"), "api")
        .unwrap()
        .collect();

    // All three should be present as resources are case-sensitive
    assert_eq!(resources.len(), 3);
    assert!(resources.contains(&"/API/users"));
    assert!(resources.contains(&"/api/USERS"));
    assert!(resources.contains(&"/api/users"));
}

#[test]
fn test_get_resources_deduplication() {
    let mut structure = IxAccessStructureV1::new();
    structure.add_role(&Role::new("user"));

    // Assign the same resource multiple times
    structure
        .assign_resource_to_role(&Role::new("user"), "db", "main_db")
        .unwrap();
    structure
        .assign_resource_to_role(&Role::new("user"), "db", "main_db")
        .unwrap();
    structure
        .assign_resource_to_role(&Role::new("user"), "db", "main_db")
        .unwrap();

    let resources: Vec<_> = structure
        .get_all_resources_for_role_by_tag(&Role::new("user"), "db")
        .unwrap()
        .collect();

    // Should be deduplicated
    assert_eq!(resources.len(), 1);
    assert_eq!(resources[0], "main_db");
}

#[test]
fn test_assign_resource_to_nonexistent_role() {
    let mut structure = IxAccessStructureV1::new();

    let result = structure.assign_resource_to_role(&Role::new("nonexistent"), "db", "test_db");
    assert!(result.is_err());
    match result {
        Err(StructureError::RoleNotFound(s)) => assert_eq!(s, "nonexistent"),
        _ => panic!("Expected RoleNotFound error"),
    }
}

#[test]
fn test_get_resources_special_characters() {
    let mut structure = IxAccessStructureV1::new();
    structure.add_role(&Role::new("service"));

    structure
        .assign_resource_to_role(
            &Role::new("service"),
            "conn",
            "postgresql://user:pass@localhost:5432/db?sslmode=require",
        )
        .unwrap();
    structure
        .assign_resource_to_role(
            &Role::new("service"),
            "path",
            "C:\\Program Files\\App\\config.json",
        )
        .unwrap();
    structure
        .assign_resource_to_role(
            &Role::new("service"),
            "regex",
            "^[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\\.[a-zA-Z]{2,}$",
        )
        .unwrap();

    let conn: Vec<_> = structure
        .get_all_resources_for_role_by_tag(&Role::new("service"), "conn")
        .unwrap()
        .collect();
    assert_eq!(conn.len(), 1);
    assert_eq!(
        conn[0],
        "postgresql://user:pass@localhost:5432/db?sslmode=require"
    );

    let path: Vec<_> = structure
        .get_all_resources_for_role_by_tag(&Role::new("service"), "path")
        .unwrap()
        .collect();
    assert_eq!(path.len(), 1);
    assert_eq!(path[0], "C:\\Program Files\\App\\config.json");

    let regex: Vec<_> = structure
        .get_all_resources_for_role_by_tag(&Role::new("service"), "regex")
        .unwrap()
        .collect();
    assert_eq!(regex.len(), 1);
    assert_eq!(
        regex[0],
        "^[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\\.[a-zA-Z]{2,}$"
    );
}

#[quickcheck_macros::quickcheck]
fn prop_bfs_no_duplicates(roles: Vec<AsciiRole>) -> TestResult {
    let roles: Vec<_> = roles.into_iter().take(5).collect();
    if roles.is_empty() {
        return TestResult::passed();
    }

    let mut structure = IxAccessStructureV1::new();
    for role in &roles {
        structure.add_role(&Role::new(&role.0));
    }

    // Assign all roles to first role
    if roles.len() > 1 {
        for i in 1..roles.len() {
            structure
                .assign_role(&Role::new(&roles[0].0), &Role::new(&roles[i].0))
                .unwrap();
        }
    }

    let result: Vec<_> = structure
        .list_all_roles_for_role(&Role::new(&roles[0].0))
        .unwrap()
        .collect();
    let unique_count = result
        .iter()
        .collect::<std::collections::HashSet<_>>()
        .len();

    TestResult::from_bool(result.len() == unique_count)
}

#[test]
fn test_unassign_role_simple() {
    let mut structure = create_test_structure();
    structure
        .assign_role(&Role::new("admin"), &Role::new("user"))
        .unwrap();

    let roles_before: Vec<_> = structure
        .list_all_roles_for_role(&Role::new("admin"))
        .unwrap()
        .collect();
    assert_eq!(roles_before.len(), 2);

    structure
        .unassign_role(&Role::new("admin"), &Role::new("user"))
        .unwrap();

    let roles_after: Vec<_> = structure
        .list_all_roles_for_role(&Role::new("admin"))
        .unwrap()
        .collect();
    assert_eq!(roles_after.len(), 1);
    assert_eq!(roles_after[0], "admin");
}

#[test]
fn test_unassign_role_chain() {
    let mut structure = create_test_structure();
    structure
        .assign_role(&Role::new("admin"), &Role::new("user"))
        .unwrap();
    structure
        .assign_role(&Role::new("user"), &Role::new("guest"))
        .unwrap();

    structure
        .unassign_role(&Role::new("admin"), &Role::new("user"))
        .unwrap();

    let roles: Vec<_> = structure
        .list_all_roles_for_role(&Role::new("admin"))
        .unwrap()
        .collect();
    assert_eq!(roles.len(), 1);
    assert_eq!(roles[0], "admin");
    assert!(!roles.contains(&"user"));
    assert!(!roles.contains(&"guest"));
}

#[test]
fn test_unassign_role_nonexistent_assignee() {
    let mut structure = create_test_structure();
    let result = structure.unassign_role(&Role::new("nonexistent"), &Role::new("user"));
    assert!(result.is_err());
    assert!(
        matches!(result.unwrap_err(), StructureError::RoleNotFound(ref s) if s == "nonexistent")
    );
}

#[test]
fn test_unassign_role_nonexistent_role() {
    let mut structure = create_test_structure();
    let result = structure.unassign_role(&Role::new("admin"), &Role::new("nonexistent"));
    assert!(result.is_err());
    assert!(
        matches!(result.unwrap_err(), StructureError::RoleNotFound(ref s) if s == "nonexistent")
    );
}

#[test]
fn test_unassign_role_not_assigned() {
    let mut structure = create_test_structure();

    // No assignment between admin and guest, but both roles exist
    let result = structure.unassign_role(&Role::new("admin"), &Role::new("guest"));
    assert!(result.is_ok());

    let roles: Vec<_> = structure
        .list_all_roles_for_role(&Role::new("admin"))
        .unwrap()
        .collect();
    assert_eq!(roles.len(), 1);
    assert_eq!(roles[0], "admin");
}

#[test]
fn test_unassign_role_multiple_assignments() {
    let mut structure = create_test_structure();
    structure
        .assign_role(&Role::new("admin"), &Role::new("user"))
        .unwrap();
    structure
        .assign_role(&Role::new("admin"), &Role::new("guest"))
        .unwrap();

    structure
        .unassign_role(&Role::new("admin"), &Role::new("user"))
        .unwrap();

    let roles: Vec<_> = structure
        .list_all_roles_for_role(&Role::new("admin"))
        .unwrap()
        .collect();
    assert_eq!(roles.len(), 2);
    assert!(roles.contains(&"admin"));
    assert!(roles.contains(&"guest"));
    assert!(!roles.contains(&"user"));
}

#[test]
fn test_unassign_role_case_insensitive() {
    let mut structure = create_test_structure();
    structure
        .assign_role(&Role::new("admin"), &Role::new("user"))
        .unwrap();

    structure
        .unassign_role(&Role::new("ADMIN"), &Role::new("USER"))
        .unwrap();

    let roles: Vec<_> = structure
        .list_all_roles_for_role(&Role::new("admin"))
        .unwrap()
        .collect();
    assert_eq!(roles.len(), 1);
    assert_eq!(roles[0], "admin");
}

#[test]
fn test_unassign_resource_simple() {
    let mut structure = IxAccessStructureV1::new();
    structure.add_role(&Role::new("user"));

    structure
        .assign_resource_to_role(&Role::new("user"), "database", "users_db")
        .unwrap();
    structure
        .assign_resource_to_role(&Role::new("user"), "database", "logs_db")
        .unwrap();

    let resources_before: Vec<_> = structure
        .get_all_resources_for_role_by_tag(&Role::new("user"), "database")
        .unwrap()
        .collect();
    assert_eq!(resources_before.len(), 2);

    structure
        .unassign_resource_from_role(&Role::new("user"), "database", "users_db")
        .unwrap();

    let resources_after: Vec<_> = structure
        .get_all_resources_for_role_by_tag(&Role::new("user"), "database")
        .unwrap()
        .collect();
    assert_eq!(resources_after.len(), 1);
    assert_eq!(resources_after[0], "logs_db");
}

#[test]
fn test_unassign_resource_all() {
    let mut structure = IxAccessStructureV1::new();
    structure.add_role(&Role::new("user"));

    structure
        .assign_resource_to_role(&Role::new("user"), "database", "users_db")
        .unwrap();

    structure
        .unassign_resource_from_role(&Role::new("user"), "database", "users_db")
        .unwrap();

    let resources: Vec<_> = structure
        .get_all_resources_for_role_by_tag(&Role::new("user"), "database")
        .unwrap()
        .collect();
    assert_eq!(resources.len(), 0);
}

#[test]
fn test_unassign_resource_nonexistent_role() {
    let mut structure = IxAccessStructureV1::new();

    let result = structure.unassign_resource_from_role(&Role::new("nonexistent"), "db", "test_db");
    assert!(result.is_err());
    match result {
        Err(StructureError::RoleNotFound(s)) => assert_eq!(s, "nonexistent"),
        _ => panic!("Expected RoleNotFound error"),
    }
}

#[test]
fn test_unassign_resource_nonexistent_tag() {
    let mut structure = IxAccessStructureV1::new();
    structure.add_role(&Role::new("user"));

    let result =
        structure.unassign_resource_from_role(&Role::new("user"), "nonexistent_tag", "value");
    assert!(result.is_err());
    match result {
        Err(StructureError::ResourceTagNotFound(s)) => assert_eq!(s, "nonexistent_tag"),
        _ => panic!("Expected ResourceTagNotFound error"),
    }
}

#[test]
fn test_unassign_resource_nonexistent_value() {
    let mut structure = IxAccessStructureV1::new();
    structure.add_role(&Role::new("user"));

    structure
        .assign_resource_to_role(&Role::new("user"), "database", "users_db")
        .unwrap();

    let result =
        structure.unassign_resource_from_role(&Role::new("user"), "database", "nonexistent");
    assert!(result.is_err());
    match result {
        Err(StructureError::ResourceTagNotFound(s)) => assert_eq!(s, "nonexistent"),
        _ => panic!("Expected ResourceTagNotFound error"),
    }
}

#[test]
fn test_unassign_resource_multiple_tags() {
    let mut structure = IxAccessStructureV1::new();
    structure.add_role(&Role::new("user"));

    structure
        .assign_resource_to_role(&Role::new("user"), "database", "users_db")
        .unwrap();
    structure
        .assign_resource_to_role(&Role::new("user"), "api", "/api/users")
        .unwrap();
    structure
        .assign_resource_to_role(&Role::new("user"), "file", "/data/users")
        .unwrap();

    structure
        .unassign_resource_from_role(&Role::new("user"), "database", "users_db")
        .unwrap();

    // Database resource should be gone
    let db_resources: Vec<_> = structure
        .get_all_resources_for_role_by_tag(&Role::new("user"), "database")
        .unwrap()
        .collect();
    assert_eq!(db_resources.len(), 0);

    // Other resources should remain
    let api_resources: Vec<_> = structure
        .get_all_resources_for_role_by_tag(&Role::new("user"), "api")
        .unwrap()
        .collect();
    assert_eq!(api_resources.len(), 1);

    let file_resources: Vec<_> = structure
        .get_all_resources_for_role_by_tag(&Role::new("user"), "file")
        .unwrap()
        .collect();
    assert_eq!(file_resources.len(), 1);
}

#[test]
fn test_unassign_resource_case_sensitive() {
    let mut structure = IxAccessStructureV1::new();
    structure.add_role(&Role::new("user"));

    structure
        .assign_resource_to_role(&Role::new("user"), "api", "/API/users")
        .unwrap();
    structure
        .assign_resource_to_role(&Role::new("user"), "api", "/api/users")
        .unwrap();

    structure
        .unassign_resource_from_role(&Role::new("user"), "api", "/API/users")
        .unwrap();

    let resources: Vec<_> = structure
        .get_all_resources_for_role_by_tag(&Role::new("user"), "api")
        .unwrap()
        .collect();
    assert_eq!(resources.len(), 1);
    assert_eq!(resources[0], "/api/users");
}

#[test]
fn test_unassign_resource_unicode() {
    let mut structure = IxAccessStructureV1::new();
    structure.add_role(&Role::new("user"));

    structure
        .assign_resource_to_role(&Role::new("user"), "file", "/path/to/café.txt")
        .unwrap();
    structure
        .assign_resource_to_role(&Role::new("user"), "url", "https://example.com/文档")
        .unwrap();

    structure
        .unassign_resource_from_role(&Role::new("user"), "file", "/path/to/café.txt")
        .unwrap();

    let files: Vec<_> = structure
        .get_all_resources_for_role_by_tag(&Role::new("user"), "file")
        .unwrap()
        .collect();
    assert_eq!(files.len(), 0);

    let urls: Vec<_> = structure
        .get_all_resources_for_role_by_tag(&Role::new("user"), "url")
        .unwrap()
        .collect();
    assert_eq!(urls.len(), 1);
    assert_eq!(urls[0], "https://example.com/文档");
}

#[test]
fn test_delete_role_leaf() {
    let mut structure = IxAccessStructureV1::new();
    structure.add_role(&Role::new("admin"));
    structure.add_role(&Role::new("editor"));
    structure
        .assign_role(&Role::new("admin"), &Role::new("editor"))
        .unwrap();

    structure.delete_role(&Role::new("editor"));

    assert!(!structure.exists_role(&Role::new("editor")));
    assert!(structure.exists_role(&Role::new("admin")));

    let roles: Vec<_> = structure
        .list_all_roles_for_role(&Role::new("admin"))
        .unwrap()
        .collect();
    assert_eq!(roles, vec!["admin"]);
}

#[test]
fn test_delete_role_parent() {
    let mut structure = IxAccessStructureV1::new();
    structure.add_role(&Role::new("admin"));
    structure.add_role(&Role::new("editor"));
    structure
        .assign_role(&Role::new("admin"), &Role::new("editor"))
        .unwrap();

    structure.delete_role(&Role::new("admin"));

    assert!(!structure.exists_role(&Role::new("admin")));
    assert!(structure.exists_role(&Role::new("editor")));
}

#[test]
fn test_delete_role_with_resources() {
    let mut structure = IxAccessStructureV1::new();
    structure.add_role(&Role::new("admin"));
    structure
        .assign_resource_to_role(&Role::new("admin"), "tag", "value")
        .unwrap();

    structure.delete_role(&Role::new("admin"));

    assert!(!structure.exists_role(&Role::new("admin")));

    // Check if resources are purged by re-adding and checking
    structure.add_role(&Role::new("admin"));
    let resources = structure
        .get_all_resources_for_role(&Role::new("admin"))
        .unwrap();
    assert!(resources.is_empty());
}

#[test]
fn test_delete_role_list_roles() {
    let mut structure = IxAccessStructureV1::new();
    structure.add_role(&Role::new("admin"));
    structure.add_role(&Role::new("editor"));

    structure.delete_role(&Role::new("admin"));

    let roles: Vec<_> = structure.list_roles().collect();
    assert_eq!(roles, vec!["editor"]);
}

#[test]
fn test_delete_role_redundant_path_preservation() {
    // Scenario: A -> B -> C and A -> C (Triangle). Delete B.
    // Expectation: A still has access to C.
    let mut structure = IxAccessStructureV1::new();
    structure.add_role(&Role::new("a"));
    structure.add_role(&Role::new("b"));
    structure.add_role(&Role::new("c"));

    structure
        .assign_role(&Role::new("a"), &Role::new("b"))
        .unwrap();
    structure
        .assign_role(&Role::new("b"), &Role::new("c"))
        .unwrap();
    structure
        .assign_role(&Role::new("a"), &Role::new("c"))
        .unwrap();

    structure.delete_role(&Role::new("b"));

    assert!(structure.has_role(&Role::new("a"), &Role::new("c")));
}

#[test]
fn test_delete_role_transitive_revocation() {
    // Scenario: A -> B -> C. Delete B.
    // Expectation: A loses access to C.
    let mut structure = IxAccessStructureV1::new();
    structure.add_role(&Role::new("a"));
    structure.add_role(&Role::new("b"));
    structure.add_role(&Role::new("c"));

    structure
        .assign_role(&Role::new("a"), &Role::new("b"))
        .unwrap();
    structure
        .assign_role(&Role::new("b"), &Role::new("c"))
        .unwrap();

    assert!(structure.has_role(&Role::new("a"), &Role::new("c")));

    structure.delete_role(&Role::new("b"));

    assert!(!structure.has_role(&Role::new("a"), &Role::new("c")));
}

#[test]
fn test_delete_role_diamond_dependency() {
    // Scenario: A -> B -> D and A -> C -> D. Delete B.
    // Expectation: A still has access to D via C.
    let mut structure = IxAccessStructureV1::new();
    structure.add_role(&Role::new("a"));
    structure.add_role(&Role::new("b"));
    structure.add_role(&Role::new("c"));
    structure.add_role(&Role::new("d"));

    structure
        .assign_role(&Role::new("a"), &Role::new("b"))
        .unwrap();
    structure
        .assign_role(&Role::new("a"), &Role::new("c"))
        .unwrap();
    structure
        .assign_role(&Role::new("b"), &Role::new("d"))
        .unwrap();
    structure
        .assign_role(&Role::new("c"), &Role::new("d"))
        .unwrap();

    structure.delete_role(&Role::new("b"));

    assert!(structure.has_role(&Role::new("a"), &Role::new("d")));
}

#[test]
fn test_delete_role_self_inheritance() {
    // Scenario: A -> A (Self-inheritance). Delete A.
    // Expectation: No panic, role is removed.
    let mut structure = IxAccessStructureV1::new();
    structure.add_role(&Role::new("a"));
    structure
        .assign_role(&Role::new("a"), &Role::new("a"))
        .unwrap();

    structure.delete_role(&Role::new("a"));

    assert!(!structure.exists_role(&Role::new("a")));
}

#[test]
fn test_delete_role_total_exhaustion() {
    // Scenario: Delete all roles one by one.
    // Expectation: Internal vectors are empty.
    let mut structure = IxAccessStructureV1::new();
    structure.add_role(&Role::new("a"));
    structure.add_role(&Role::new("b"));
    structure
        .assign_role(&Role::new("a"), &Role::new("b"))
        .unwrap();

    structure.delete_role(&Role::new("a"));
    structure.delete_role(&Role::new("b"));

    assert_eq!(structure.list_roles().count(), 0);
    assert_eq!(structure.role_graph.len(), 0);
    assert_eq!(structure.resource_assignment.len(), 0);
}

#[test]
fn test_delete_role_boundary_elements() {
    // Scenario: Delete first and last elements specifically.
    // Expectation: Mapping handles boundaries correctly.
    let mut structure = IxAccessStructureV1::new();
    structure.add_role(&Role::new("first"));
    structure.add_role(&Role::new("middle"));
    structure.add_role(&Role::new("last"));

    // Delete first
    structure.delete_role(&Role::new("first"));
    assert_eq!(
        structure.list_roles().collect::<Vec<_>>(),
        vec!["middle", "last"]
    );

    // Delete last
    structure.delete_role(&Role::new("last"));
    assert_eq!(structure.list_roles().collect::<Vec<_>>(), vec!["middle"]);
}

#[test]
fn test_delete_role_preserves_unrelated_inheritance() {
    let mut structure = IxAccessStructureV1::new();
    structure.add_role(&Role::new("a"));
    structure.add_role(&Role::new("b"));
    structure.add_role(&Role::new("c"));
    structure.add_role(&Role::new("to_delete"));

    structure
        .assign_role(&Role::new("a"), &Role::new("b"))
        .unwrap();
    structure
        .assign_role(&Role::new("b"), &Role::new("c"))
        .unwrap();
    structure
        .assign_role(&Role::new("to_delete"), &Role::new("c"))
        .unwrap();

    structure.delete_role(&Role::new("to_delete"));

    assert!(structure.has_role(&Role::new("a"), &Role::new("c")));
    assert!(structure.has_role(&Role::new("b"), &Role::new("c")));
}

#[test]
fn test_delete_role_preserves_unrelated_resources() {
    let mut structure = IxAccessStructureV1::new();
    structure.add_role(&Role::new("keep"));
    structure.add_role(&Role::new("drop"));

    structure
        .assign_resource_to_role(&Role::new("keep"), "tag", "val1")
        .unwrap();
    structure
        .assign_resource_to_role(&Role::new("drop"), "tag", "val2")
        .unwrap();

    structure.delete_role(&Role::new("drop"));

    let resources: Vec<_> = structure
        .get_all_resources_for_role_by_tag(&Role::new("keep"), "tag")
        .unwrap()
        .collect();
    assert_eq!(resources, vec!["val1"]);
}

#[test]
fn test_delete_role_cleans_up_incoming_edges() {
    let mut structure = IxAccessStructureV1::new();
    structure.add_role(&Role::new("parent"));
    structure.add_role(&Role::new("child"));
    structure
        .assign_role(&Role::new("parent"), &Role::new("child"))
        .unwrap();

    structure.delete_role(&Role::new("child"));

    // Parent should still exist but have no children
    assert!(structure.exists_role(&Role::new("parent")));
    let children: Vec<_> = structure
        .list_all_roles_for_role(&Role::new("parent"))
        .unwrap()
        .collect();
    assert_eq!(children, vec!["parent"]);
}

#[test]
fn test_delete_nonexistent_role_is_noop() {
    let mut structure = IxAccessStructureV1::new();
    structure.add_role(&Role::new("a"));
    structure.delete_role(&Role::new("b"));
    assert!(structure.exists_role(&Role::new("a")));
    assert_eq!(structure.list_roles().count(), 1);
}

#[test]
fn test_delete_and_readd_role() {
    let mut structure = IxAccessStructureV1::new();
    structure.add_role(&Role::new("admin"));
    structure
        .assign_resource_to_role(&Role::new("admin"), "tag", "value")
        .unwrap();

    structure.delete_role(&Role::new("admin"));
    structure.add_role(&Role::new("admin"));

    assert!(structure.exists_role(&Role::new("admin")));
    let resources = structure
        .get_all_resources_for_role(&Role::new("admin"))
        .unwrap();
    assert!(resources.is_empty());
}
