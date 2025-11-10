.onLoad <- function(...) {
  S7::methods_register()
}

#' IxAccessClient S7 Class
#'
#' An S7 wrapper around the internal IxAccessClientInternal extendr object.
#'
#' @export
IxAccessClient <- S7::new_class(
  "IxAccessClient",
  properties = list(
    internal = S7::class_any
  ),
  constructor = function(path) {
    internal <- IxAccessClientInternal$new(path)
    S7::new_object(
      S7::S7_object(),
      internal = internal
    )
  }
)


#' List all roles
#'
#' @param client An IxAccessClient object
#' @return A character vector of role names
#' @export
list_roles <- S7::new_generic("list_roles", "client")

#' @export
S7::method(list_roles, IxAccessClient) <- function(client) {
  client@internal$list_roles()
}

#' List all roles for a given role
#'
#' @param client An IxAccessClient object
#' @param role The role name
#' @return A character vector of role names
#' @export
list_all_roles_for_role <- S7::new_generic("list_all_roles_for_role", "client")

#' @export
S7::method(list_all_roles_for_role, IxAccessClient) <- function(client, role) {
  client@internal$list_all_roles_for_role(role)
}

#' Add a role
#'
#' @param client An IxAccessClient object
#' @param role The role name to add
#' @return NULL (invisibly)
#' @export
add_role <- S7::new_generic("add_role", "client")

#' @export
S7::method(add_role, IxAccessClient) <- function(client, role) {
  client@internal$add_role(role)
  invisible(NULL)
}

#' Add multiple roles
#'
#' @param client An IxAccessClient object
#' @param roles A character vector of role names to add
#' @return NULL (invisibly)
#' @export
add_roles <- S7::new_generic("add_roles", "client")

#' @export
S7::method(add_roles, IxAccessClient) <- function(client, roles) {
  client@internal$add_roles(roles)
  invisible(NULL)
}

#' Assign a role to another role
#'
#' @param client An IxAccessClient object
#' @param assignee The role receiving the assignment
#' @param role The role being assigned
#' @return NULL (invisibly)
#' @export
assign_role <- S7::new_generic("assign_role", "client")

#' @export
S7::method(assign_role, IxAccessClient) <- function(client, assignee, role) {
  client@internal$assign_role(assignee, role)
  invisible(NULL)
}

#' Assign a resource to a role
#'
#' @param client An IxAccessClient object
#' @param role The role name
#' @param resource_tag The resource tag
#' @param resource_value The resource value
#' @return NULL (invisibly)
#' @export
assign_resource_to_role <- S7::new_generic("assign_resource_to_role", "client")

#' @export
S7::method(assign_resource_to_role, IxAccessClient) <- function(
  client,
  role,
  resource_tag,
  resource_value
) {
  client@internal$assign_resource_to_role(role, resource_tag, resource_value)
  invisible(NULL)
}

#' Get all resources for a role by tag
#'
#' @param client An IxAccessClient object
#' @param role The role name
#' @param tag The resource tag
#' @return A character vector of resource values
#' @export
get_all_resources_for_role_by_tag <- S7::new_generic(
  "get_all_resources_for_role_by_tag",
  "client"
)

#' @export
S7::method(get_all_resources_for_role_by_tag, IxAccessClient) <- function(
  client,
  role,
  tag
) {
  client@internal$get_all_resources_for_role_by_tag(role, tag)
}

#' Unassign a role from another role
#'
#' @param client An IxAccessClient object
#' @param assignee The role to remove the assignment from
#' @param role The role being unassigned
#' @return NULL (invisibly)
#' @export
unassign_role <- S7::new_generic("unassign_role", "client")

#' @export
S7::method(unassign_role, IxAccessClient) <- function(client, assignee, role) {
  client@internal$unassign_role(assignee, role)
  invisible(NULL)
}

#' Unassign a resource from a role
#'
#' @param client An IxAccessClient object
#' @param role The role name
#' @param resource_tag The resource tag
#' @param resource_value The resource value
#' @return NULL (invisibly)
#' @export
unassign_resource_from_role <- S7::new_generic(
  "unassign_resource_from_role",
  "client"
)

#' @export
S7::method(unassign_resource_from_role, IxAccessClient) <- function(
  client,
  role,
  resource_tag,
  resource_value
) {
  client@internal$unassign_resource_from_role(
    role,
    resource_tag,
    resource_value
  )
  invisible(NULL)
}
