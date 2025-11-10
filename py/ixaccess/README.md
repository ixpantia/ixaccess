# IxAccess Python Bindings

Python bindings for IxAccess - a library for managing access control roles and resources in a hierarchical structure.

IxAccess stores its state in a single file on a cloud object store (like GCS, S3, or Azure Blob Storage), making it easy to share access control policies across multiple services and environments.

## Features

- **In-memory, thread-safe cache** for fast read access
- **Automatic cache invalidation** when the remote file changes
- **Atomic write operations** to prevent race conditions
- Support for **Google Cloud Storage**, **AWS S3**, **Azure Blob Storage**, and **local files**

## Installation

### From Source

```bash
cd py/ixaccess
pip install maturin
maturin develop  # For development
# or
maturin build --release  # For production builds
pip install target/wheels/*.whl
```

## Cloud Storage and Authentication

The client uses URLs to specify the location of the state file in the object store:

- **Google Cloud Storage:** `gs://<bucket>/<path>`
- **Amazon S3:** `s3://<bucket>/<path>`
- **Azure Blob Storage:** `az://<container>/<path>`
- **Local file:** `/path/to/file` or `file:///path/to/file`

### Authentication

Authentication is handled automatically using standard cloud provider credentials:

#### Google Cloud Platform
- `GOOGLE_APPLICATION_CREDENTIALS` environment variable
- Application Default Credentials (ADC)
- Service account on GCP services (GCE, GKE, Cloud Run)

#### Amazon Web Services
- `AWS_ACCESS_KEY_ID`, `AWS_SECRET_ACCESS_KEY` environment variables
- `~/.aws/credentials` file
- IAM roles for EC2/ECS

#### Microsoft Azure
- `AZURE_STORAGE_ACCOUNT`, `AZURE_STORAGE_ACCESS_KEY` environment variables
- Managed identity on Azure services

## Usage

```python
from ixaccess import IxAccessClient

# Initialize the client with a path to the state file
client = IxAccessClient("gs://your-bucket/access-control.ix")

# --- Role Management ---
print("Adding roles...")
client.add_role("admin")
client.add_role("editor")
client.add_role("viewer")

# --- Role Assignment (Inheritance) ---
# 'admin' inherits all permissions from 'editor'
# 'editor' inherits all permissions from 'viewer'
print("Assigning roles...")
client.assign_role("admin", "editor")
client.assign_role("editor", "viewer")

# --- Resource Assignment ---
# Assign a GCS bucket to the 'viewer' role
print("Assigning resources...")
client.assign_resource_to_role("viewer", "gcs_bucket", "data-bucket-1")

# --- Access Checks ---
print("Performing access checks...")

# The 'admin' role has access because it inherits from 'viewer'
admin_buckets = client.get_all_resources_for_role_by_tag("admin", "gcs_bucket")
assert "data-bucket-1" in admin_buckets
print(f"'admin' has access to: {admin_buckets}")

# The 'viewer' role has direct access
viewer_buckets = client.get_all_resources_for_role_by_tag("viewer", "gcs_bucket")
assert "data-bucket-1" in viewer_buckets
print(f"'viewer' has access to: {viewer_buckets}")

# --- List Operations ---
all_roles = client.list_roles()
print(f"All roles: {all_roles}")

admin_inherited_roles = client.list_all_roles_for_role("admin")
print(f"Roles inherited by admin: {admin_inherited_roles}")
```

## API Reference

### IxAccessClient

#### `__init__(path: str)`
Creates a new IxAccessClient instance.

**Parameters:**
- `path`: URL-style path to the state file (e.g., "gs://my-bucket/access-control.ix")

#### `list_roles() -> List[str]`
Lists all top-level roles.

#### `list_all_roles_for_role(role: str) -> List[str]`
Lists all roles assigned to a given role, including nested roles.

#### `add_role(role: str) -> None`
Adds a new role to the access control structure.

#### `add_roles(roles: List[str]) -> None`
Adds multiple roles at once (more efficient than calling `add_role` in a loop).

#### `assign_role(assignee: str, role: str) -> None`
Assigns a role to another role (the assignee). Creates a parent-child relationship where assignee inherits permissions from role.

#### `assign_resource_to_role(role: str, resource_tag: str, resource_value: str) -> None`
Assigns a resource to a role.

**Parameters:**
- `role`: The role to assign the resource to
- `resource_tag`: The type/tag of the resource (e.g., "gcs_bucket")
- `resource_value`: The specific resource identifier (e.g., "my-data-bucket")

#### `get_all_resources_for_role_by_tag(role: str, tag: str) -> List[str]`
Gets all resources with a specific tag that are assigned to a role. Includes resources from inherited roles.

#### `unassign_role(assignee: str, role: str) -> None`
Removes a role assignment.

#### `unassign_resource_from_role(role: str, resource_tag: str, resource_value: str) -> None`
Removes a resource assignment from a role.

## Development

### Building

```bash
# Install development dependencies
pip install maturin

# Build in development mode
maturin develop

# Build release wheels
maturin build --release

# Run tests (if available)
pytest
```

### Requirements

- Python >= 3.8
- Rust toolchain (for building from source)
- maturin >= 1.0

## License

See the main project LICENSE file.