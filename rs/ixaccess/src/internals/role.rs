/// A role identifier that is guaranteed to be lowercase ASCII.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Role(Box<str>);

impl Role {
    /// Creates a new Role from a string, converting it to lowercase ASCII.
    ///
    /// # Panics
    /// Panics if the input contains non-ASCII characters.
    pub fn new(role: impl AsRef<str>) -> Self {
        let role_str = role.as_ref().trim();
        assert!(role_str.is_ascii(), "Role must be ASCII");
        let lowercase = role_str.to_ascii_lowercase();
        Role(lowercase.into_boxed_str())
    }

    /// Returns the role as a string slice.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl AsRef<str> for Role {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use quickcheck::{Arbitrary, Gen};

    #[test]
    fn test_role_new_converts_to_lowercase() {
        let role = Role::new("ADMIN");
        assert_eq!(role.as_str(), "admin");

        let role = Role::new("User");
        assert_eq!(role.as_str(), "user");

        let role = Role::new("guest");
        assert_eq!(role.as_str(), "guest");
    }

    #[test]
    fn test_role_trims_whitespace() {
        let role1 = Role::new("  admin  ");
        assert_eq!(role1.as_str(), "admin");

        let role2 = Role::new("\tuser\t");
        assert_eq!(role2.as_str(), "user");

        let role3 = Role::new("  guest");
        assert_eq!(role3.as_str(), "guest");

        let role4 = Role::new("moderator  ");
        assert_eq!(role4.as_str(), "moderator");
    }

    #[test]
    fn test_role_as_ref_str() {
        let role = Role::new("test");
        let s: &str = role.as_ref();
        assert_eq!(s, "test");
    }

    #[test]
    #[should_panic(expected = "Role must be ASCII")]
    fn test_role_panics_on_non_ascii() {
        Role::new("café");
    }

    #[test]
    fn test_role_equality() {
        let role1 = Role::new("admin");
        let role2 = Role::new("ADMIN");
        assert_eq!(role1, role2);
    }

    #[test]
    fn test_role_clone() {
        let role1 = Role::new("admin");
        let role2 = role1.clone();
        assert_eq!(role1, role2);
    }

    #[test]
    fn test_role_hash() {
        use std::collections::HashSet;
        let mut set = HashSet::new();
        set.insert(Role::new("admin"));
        set.insert(Role::new("ADMIN"));
        set.insert(Role::new("Admin"));

        // All should be the same due to lowercase conversion
        assert_eq!(set.len(), 1);
    }

    #[derive(Clone, Debug)]
    pub(crate) struct AsciiRole(pub String);

    impl Arbitrary for AsciiRole {
        fn arbitrary(g: &mut Gen) -> Self {
            let size = usize::arbitrary(g) % 20 + 1; // 1-20 characters
            let s: String = (0..size)
                .map(|_| {
                    let chars = b"abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789-_";
                    chars[usize::arbitrary(g) % chars.len()] as char
                })
                .collect();
            AsciiRole(s)
        }
    }

    #[quickcheck_macros::quickcheck]
    fn prop_role_always_lowercase(role_str: AsciiRole) -> bool {
        let role = Role::new(&role_str.0);
        role.as_str() == role_str.0.to_ascii_lowercase()
    }

    #[quickcheck_macros::quickcheck]
    fn prop_role_case_insensitive(role_str: AsciiRole) -> bool {
        let lower = role_str.0.to_ascii_lowercase();
        let upper = role_str.0.to_ascii_uppercase();

        let role1 = Role::new(&lower);
        let role2 = Role::new(&upper);

        role1 == role2
    }
}
