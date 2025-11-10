"""
Example script demonstrating the IxAccess Python bindings.

This script shows how to:
1. Initialize the IxAccessClient
2. Create roles
3. Set up role hierarchies (inheritance)
4. Assign resources to roles
5. Query access permissions
"""

from ixaccess import IxAccessClient


def main():
    client = IxAccessClient("./example-access-control.ix")

    client.add_role("admin")
    client.add_role("editor")
    client.add_role("viewer")

    client.assign_role("admin", "editor")
    client.assign_role("editor", "viewer")

    client.assign_resource_to_role("viewer", "gcs_bucket", "data-bucket-1")
    client.assign_resource_to_role("viewer", "gcs_bucket", "logs-bucket-1")

    client.assign_resource_to_role("editor", "gcs_bucket", "config-bucket-1")
    client.assign_resource_to_role("editor", "database", "analytics-db")

    client.assign_resource_to_role("admin", "gcs_bucket", "secrets-bucket")
    client.assign_resource_to_role("admin", "database", "master-db")

    all_roles = client.list_roles()
    print(all_roles)

    admin_roles = client.list_all_roles_for_role("admin")
    print(admin_roles)

    editor_roles = client.list_all_roles_for_role("editor")
    print(editor_roles)

if __name__ == "__main__":
    main()
