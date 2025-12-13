# IxAccess

IxAccess is a library for managing access control roles and resources in a
hierarchical structure. It stores its state in a single file on a cloud object
store (like GCS, S3, or Azure Blob Storage), making it easy to share access
control policies across multiple services and environments.

The client is designed for high performance and safety, featuring:
*   An in-memory, thread-safe cache for fast read access.
*   Automatic cache invalidation when the remote file changes.
*   Atomic write operations to prevent race conditions.

## Cloud Storage and Authentication

The client uses URLs to specify the location of the state file in the object
store. The URL scheme determines the cloud provider:

- **Google Cloud Storage:** `gs://<bucket>/<path>`
- **Amazon S3:** `s3://<bucket>/<path>`
- **Azure Blob Storage:** `az://<container>/<path>`
- **Local file:** `/path/to/file` or `file:///path/to/file`

Authentication is handled automatically by the underlying `object_store` crate,
which uses the standard environment variables and credential resolution methods
for each cloud provider.

### Google Cloud Platform

The client will use the Application Default Credentials (ADC). You can provide
credentials by:

- Setting the `GOOGLE_APPLICATION_CREDENTIALS` environment variable to the path
  of a service account key file.
- Running on a GCP service (e.g., GCE, GKE, Cloud Run) with a service account
  attached.
- Authenticating with the gcloud CLI using `gcloud auth application-default
  login`.

### Amazon Web Services

The client will use the default credential provider chain. This typically
involves:

- `AWS_ACCESS_KEY_ID`, `AWS_SECRET_ACCESS_KEY`, and `AWS_SESSION_TOKEN`
  environment variables.
- The `~/.aws/credentials` and `~/.aws/config` files.
- IAM roles for EC2 instances or ECS tasks.

### Microsoft Azure

The client will use the default credential provider chain. This typically
involves:
- `AZURE_STORAGE_ACCOUNT` and `AZURE_STORAGE_ACCESS_KEY` environment variables.
- Managed identity when running on Azure services.

## Rust Usage

Add the following to your `Cargo.toml`:

```toml
[dependencies]
ixaccess = { path = "rs/ixaccess" } # Or use the version from crates.io
tokio = { version = "1", features = ["full"] }
```

### Example

```rust
use ixaccess::IxAccessClient;

#[tokio::main]
async fn main() {
    // Initialize the client with a path to the state file in Google Cloud Storage.
    // This will create the file if it doesn't exist.
    let client = IxAccessClient::new("gs://your-bucket/access-control.ix").await;

    // --- Role Management ---
    println!("Adding roles...");
    client.add_role("admin").await.unwrap();
    client.add_role("editor").await.unwrap();
    client.add_role("viewer").await.unwrap();

    // --- Role Assignment (Inheritance) ---
    // 'admin' inherits all permissions from 'editor'.
    // 'editor' inherits all permissions from 'viewer'.
    println!("Assigning roles...");
    client.assign_role("admin", "editor").await.unwrap();
    client.assign_role("editor", "viewer").await.unwrap();

    // --- Resource Assignment ---
    // Assign a GCS bucket to the 'viewer' role.
    println!("Assigning resources...");
    client
        .assign_resource_to_role("viewer", "gcs_bucket", "data-bucket-1")
        .await
        .unwrap();

    // --- Access Checks ---
    println!("Performing access checks...");
    // The 'admin' role has access because it inherits from 'viewer'.
    let admin_buckets = client
        .get_all_resources_for_role_by_tag("admin", "gcs_bucket")
        .await
        .unwrap();
    assert!(admin_buckets.contains(&"data-bucket-1".to_string()));
    println!("'admin' has access to: {:?}", admin_buckets);

    // The 'viewer' role has direct access.
    let viewer_buckets = client
        .get_all_resources_for_role_by_tag("viewer", "gcs_bucket")
        .await
        .unwrap();
    assert!(viewer_buckets.contains(&"data-bucket-1".to_string()));
    println!("'viewer' has access to: {:?}", viewer_buckets);
}
```

## Python Usage

To install ixaccess you will need to be able to compile rust code. Make sure you
have Rust installed (https://www.rust-lang.org/tools/install).

After you have set up your project with `uv init` you can add `ixaccess` using
the following:

```bash
uv add git+https://github.com/ixpantia/ixaccess.git@<version> --subdirectory py/ixaccess

## for examples for the version tagged with v0.1.0
uv add git+https://github.com/ixpantia/ixaccess.git@v0.1.0 --subdirectory py/ixaccess
```

This will install the IxAccess Rust binary and add the Python bindings to be
used in your project.

### Example

```python
from ixaccess import IxAccessClient

# Initialize the client with a path to the state file in Google Cloud Storage.
# This will create the file if it doesn't exist.
client = IxAccessClient("gs://your-bucket/access-control.ix")

# --- Role Management ---
print("Adding roles...")
client.add_role("admin")
client.add_role("editor")
client.add_role("viewer")

# --- Role Assignment (Inheritance) ---
# 'admin' inherits all permissions from 'editor'.
# 'editor' inherits all permissions from 'viewer'.
print("Assigning roles...")
client.assign_role("admin", "editor")
client.assign_role("editor", "viewer")

# --- Resource Assignment ---
# Assign a GCS bucket to the 'viewer' role.
print("Assigning resources...")
client.assign_resource_to_role("viewer", "gcs_bucket", "data-bucket-1")

# --- Access Checks ---
print("Performing access checks...")
# The 'admin' role has access because it inherits from 'viewer'.
admin_buckets = client.get_all_resources_for_role_by_tag("admin", "gcs_bucket")
assert "data-bucket-1" in admin_buckets
print(f"'admin' has access to: {admin_buckets}")

# The 'viewer' role has direct access.
viewer_buckets = client.get_all_resources_for_role_by_tag("viewer", "gcs_bucket")
assert "data-bucket-1" in viewer_buckets
print(f"'viewer' has access to: {viewer_buckets}")
```

### Python development

If you want to develop IxAccess with python note that the Python bindings are
built using PyO3 and maturin. Install from the local source:

```bash
# From the `ixaccess` project root directory
cd py/ixaccess
pip install maturin
maturin develop
```

Or build and install a wheel:

```bash
cd py/ixaccess
pip install maturin
maturin build --release
pip install target/wheels/*.whl
```

## R Usage
To install ixaccess you will need to be able to compile rust code. Make sure you
have Rust installed (https://www.rust-lang.org/tools/install).

After you have set up your project with `rv init` you can add `ixaccess` using
the following:

```bash
rv add git+https://github.com/ixpantia/ixaccess.git@<version> --subdirectory r/ixaccess

## for examples for the version tagged with v0.1.0
rv add git+https://github.com/ixpantia/ixaccess.git@v0.1.0 --subdirectory r/ixaccess
```

This will install the IxAccess Rust binary and add the R package to be used in
your project.

### Example

```r
library(ixaccess)

# Initialize the client with a path to the state file
client <- IxAccessClient("gs://your-bucket/access-control.ix")

# --- Role Management ---
print("Adding roles...")
add_role(client, "admin")
add_role(client, "editor")
add_role(client, "viewer")

# --- Role Assignment (Inheritance) ---
print("Assigning roles...")
assign_role(client, "admin", "editor")
assign_role(client, "editor", "viewer")

# --- Resource Assignment ---
print("Assigning resources...")
assign_resource_to_role(client, "viewer", "gcs_bucket", "data-bucket-1")

# --- Access Checks ---
print("Performing access checks...")

# The 'admin' role has access because it inherits from 'viewer'.
admin_buckets <- get_all_resources_for_role_by_tag(client, "admin", "gcs_bucket")
stopifnot("data-bucket-1" %in% admin_buckets)
print(paste("'admin' has access to:", paste(admin_buckets, collapse=", ")))

# The 'viewer' role has direct access.
viewer_buckets <- get_all_resources_for_role_by_tag(client, "viewer", "gcs_bucket")
stopifnot("data-bucket-1" %in% viewer_buckets)
print(paste("'viewer' has access to:", paste(viewer_buckets, collapse=", ")))
```
### R Development

Install the R package from the local source for development you can do the
following. This will include any changes to the cargo code in the compile step.

```r
# From the `ixaccess` project root directory
setwd("r/ixaccess")
devtools::install()
```
