"""
IxAccess - A library for managing access control roles and resources in a hierarchical structure.

IxAccess stores its state in a single file on a cloud object store (like GCS, S3, or Azure Blob Storage),
making it easy to share access control policies across multiple services and environments.

Example:
    >>> from ixaccess import IxAccessClient
    >>>
    >>> # Initialize the client
    >>> client = IxAccessClient("gs://your-bucket/access-control.ix")
    >>>
    >>> # Add roles
    >>> client.add_role("admin")
    >>> client.add_role("editor")
    >>> client.add_role("viewer")
    >>>
    >>> # Create role hierarchy
    >>> client.assign_role("admin", "editor")
    >>> client.assign_role("editor", "viewer")
    >>>
    >>> # Assign resources
    >>> client.assign_resource_to_role("viewer", "gcs_bucket", "data-bucket-1")
    >>>
    >>> # Check access
    >>> buckets = client.get_all_resources_for_role_by_tag("admin", "gcs_bucket")
    >>> print(buckets)
    ['data-bucket-1']
"""

from .ixaccess import IxAccessClient

__version__ = "0.1.0"
__all__ = ["IxAccessClient"]
