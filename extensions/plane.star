# Plane.so MCP Server Extension
# Provides project management tools via Plane API

# Configuration helper
def get_plane_config():
    """Get Plane API configuration from environment or defaults"""

    return {
        "api_key": env.get("PLANE_API_KEY", ""),
        "workspace_slug": env.get("PLANE_WORKSPACE_SLUG", ""),
        "base_url": env.get("PLANE_BASE_URL", "https://api.plane.so"),
    }

# API helper function
def call_plane_api(method, endpoint, body = None):
    """Make an authenticated request to the Plane API"""
    config = get_plane_config()

    if not config["api_key"]:
        return error_response("Plane API key not configured. Set PLANE_API_KEY environment variable.")

    url = "{}/api/v1/workspaces/{}/{}".format(
        config["base_url"],
        config["workspace_slug"],
        endpoint.lstrip("/"),
    )

    headers = {
        "X-API-Key": config["api_key"],
        "Content-Type": "application/json",
        "Accept": "application/json",
    }

    # Make the HTTP request based on method
    if method == "GET":
        response = http.get(url, None, headers, None)
    elif method == "POST":
        body_json = json.encode(body) if body else "{}"
        response = http.post(url, None, headers, body_json, None, None, None)
    elif method == "PUT":
        body_json = json.encode(body) if body else "{}"
        response = http.put(url, None, headers, body_json, None, None, None)
    elif method == "DELETE":
        response = http.delete(url, None, headers, None)
    else:
        return error_response("Unsupported HTTP method: {}".format(method))

    # Check if the request was successful based on status code
    status_code = response.get("status_code", 0)
    if status_code < 200 or status_code >= 300:
        error_msg = "API request failed with status {}: {}".format(
            status_code,
            response.get("body", "No error message"),
        )
        return error_response(error_msg)

    # Return the JSON response if available, otherwise parse body
    if response.get("json"):
        return response["json"]

    # Fallback to parsing body as JSON
    try_parse = response.get("body", "{}")
    if try_parse.strip():
        return json.decode(try_parse)
    return {}

# Tool implementations
def list_projects(params):
    """List all projects in the workspace"""
    result = call_plane_api("GET", "projects/")

    if type(result) == "dict" and result.get("isError"):
        return result

    # Format the projects list - handle paginated response
    if type(result) == "dict" and "results" in result:
        projects = result.get("results", [])
    elif type(result) == "list":
        projects = result
    else:
        projects = []

    output = "Found {} project(s):\n\n".format(len(projects))
    for project in projects:
        output += "📁 {} ({})\n".format(
            project.get("name", "Unnamed"),
            project.get("identifier", ""),
        )
        if project.get("description"):
            output += "   {}\n".format(project["description"][:100])
        output += "   ID: {}\n".format(project.get("id", ""))
        output += "\n"

    return {
        "content": [{"type": "text", "text": output}],
    }

def get_project(params):
    """Get details about a specific project"""
    project_id = params.get("project_id", "")

    if not project_id:
        return error_response("project_id parameter is required")

    result = call_plane_api("GET", "projects/{}/".format(project_id))

    if type(result) == "dict" and result.get("isError"):
        return result

    # Format project details
    output = "Project Details:\n"
    output += "=" * 50 + "\n\n"
    output += "Name: {}\n".format(result.get("name", ""))
    output += "Identifier: {}\n".format(result.get("identifier", ""))
    output += "Description: {}\n".format(result.get("description", "N/A"))
    output += "ID: {}\n".format(result.get("id", ""))

    if result.get("created_at"):
        output += "Created: {}\n".format(result["created_at"])

    if result.get("lead"):
        output += "Lead: {}\n".format(result["lead"])

    return {
        "content": [{"type": "text", "text": output}],
    }

def create_issue(params):
    """Create a new issue in a project"""
    project_id = params.get("project_id", "")
    name = params.get("name", "")

    if not project_id:
        return error_response("project_id parameter is required")

    if not name:
        return error_response("name parameter is required")

    # Build issue payload
    issue_data = {
        "name": name,
        "project": project_id,
    }

    # Add optional fields if provided
    if params.get("description_html"):
        issue_data["description_html"] = params["description_html"]

    if params.get("priority"):
        issue_data["priority"] = params["priority"]

    if params.get("state_id"):
        issue_data["state"] = params["state_id"]

    if params.get("assignees"):
        # Parse assignees - expect comma-separated string
        assignees = params["assignees"].split(",")
        issue_data["assignees"] = [a.strip() for a in assignees]

    result = call_plane_api("POST", "projects/{}/issues/".format(project_id), issue_data)

    if type(result) == "dict" and result.get("isError"):
        return result

    # Format success response
    output = "✅ Issue created successfully!\n\n"
    output += "Issue ID: {}\n".format(result.get("id", ""))
    output += "Name: {}\n".format(result.get("name", ""))
    output += "Sequence ID: {}\n".format(result.get("sequence_id", ""))

    return {
        "content": [{"type": "text", "text": output}],
    }

def list_issues(params):
    """List issues in a project"""
    project_id = params.get("project_id", "")

    if not project_id:
        return error_response("project_id parameter is required")

    # Build query parameters
    query_params = []
    if params.get("state_id"):
        query_params.append("state={}".format(params["state_id"]))
    if params.get("priority"):
        query_params.append("priority={}".format(params["priority"]))
    if params.get("assignee_id"):
        query_params.append("assignees={}".format(params["assignee_id"]))
    if params.get("limit"):
        query_params.append("limit={}".format(params["limit"]))

    endpoint = "projects/{}/issues/".format(project_id)
    if query_params:
        endpoint += "?" + "&".join(query_params)

    result = call_plane_api("GET", endpoint)

    if type(result) == "dict" and result.get("isError"):
        return result

    # Format issues list
    issues = result.get("results", []) if type(result) == "dict" else result
    if type(issues) != "list":
        issues = []

    output = "Found {} issue(s):\n\n".format(len(issues))
    for issue in issues:
        priority_emoji = {
            "urgent": "🔴",
            "high": "🟠",
            "medium": "🟡",
            "low": "🟢",
            "none": "⚪",
        }.get(issue.get("priority", "none"), "⚪")

        output += "{} {} - {}\n".format(
            priority_emoji,
            issue.get("sequence_id", ""),
            issue.get("name", "Untitled"),
        )

        if issue.get("assignees"):
            output += "   Assignees: {}\n".format(", ".join(issue["assignees"]))

        state = issue.get("state_detail", {})
        if state:
            output += "   State: {}\n".format(state.get("name", "Unknown"))

        output += "   ID: {}\n".format(issue.get("id", ""))
        output += "\n"

    return {
        "content": [{"type": "text", "text": output}],
    }

def get_issue(params):
    """Get details about a specific issue"""
    project_id = params.get("project_id", "")
    issue_id = params.get("issue_id", "")

    if not project_id:
        return error_response("project_id parameter is required")

    if not issue_id:
        return error_response("issue_id parameter is required")

    result = call_plane_api("GET", "projects/{}/issues/{}/".format(project_id, issue_id))

    if type(result) == "dict" and result.get("isError"):
        return result

    # Format issue details
    output = "Issue Details:\n"
    output += "=" * 50 + "\n\n"
    output += "Name: {}\n".format(result.get("name", ""))
    output += "Sequence ID: {}\n".format(result.get("sequence_id", ""))
    output += "Priority: {}\n".format(result.get("priority", "none"))

    state = result.get("state_detail", {})
    if state:
        output += "State: {}\n".format(state.get("name", "Unknown"))

    if result.get("description_html"):
        # Strip HTML tags for display (simple approach)
        desc = result["description_html"]
        desc = desc.replace("<p>", "").replace("</p>", "\n")
        desc = desc.replace("<br>", "\n")
        output += "\nDescription:\n{}\n".format(desc[:500])

    if result.get("assignees"):
        output += "\nAssignees: {}\n".format(", ".join(result["assignees"]))

    output += "\nID: {}\n".format(result.get("id", ""))

    return {
        "content": [{"type": "text", "text": output}],
    }

def update_issue(params):
    """Update an existing issue"""
    project_id = params.get("project_id", "")
    issue_id = params.get("issue_id", "")

    if not project_id:
        return error_response("project_id parameter is required")

    if not issue_id:
        return error_response("issue_id parameter is required")

    # Build update payload
    update_data = {}

    if params.get("name"):
        update_data["name"] = params["name"]

    if params.get("description_html"):
        update_data["description_html"] = params["description_html"]

    if params.get("priority"):
        update_data["priority"] = params["priority"]

    if params.get("state_id"):
        update_data["state"] = params["state_id"]

    if params.get("assignees"):
        assignees = params["assignees"].split(",")
        update_data["assignees"] = [a.strip() for a in assignees]

    if not update_data:
        return error_response("No fields to update provided")

    result = call_plane_api("PATCH", "projects/{}/issues/{}/".format(project_id, issue_id), update_data)

    if type(result) == "dict" and result.get("isError"):
        return result

    output = "✅ Issue updated successfully!\n\n"
    output += "Updated fields:\n"
    for key in update_data.keys():
        output += "  - {}\n".format(key)

    return {
        "content": [{"type": "text", "text": output}],
    }

# Helper functions
def error_response(message):
    """Create an error response"""
    return {
        "content": [{"type": "text", "text": "Error: " + message}],
        "isError": True,
    }

# Extension definition
def describe_extension():
    """Define the Plane MCP extension"""
    return Extension(
        name = "plane",
        version = "1.0.0",
        description = "Plane.so project management integration",
        tools = [
            Tool(
                name = "plane_list_projects",
                description = "List all projects in the Plane workspace",
                parameters = [],
                handler = list_projects,
            ),
            Tool(
                name = "plane_get_project",
                description = "Get details about a specific project",
                parameters = [
                    ToolParameter(
                        name = "project_id",
                        param_type = "string",
                        required = True,
                        description = "The ID of the project to retrieve",
                    ),
                ],
                handler = get_project,
            ),
            Tool(
                name = "plane_create_issue",
                description = "Create a new issue in a project",
                parameters = [
                    ToolParameter(
                        name = "project_id",
                        param_type = "string",
                        required = True,
                        description = "The ID of the project",
                    ),
                    ToolParameter(
                        name = "name",
                        param_type = "string",
                        required = True,
                        description = "The name/title of the issue",
                    ),
                    ToolParameter(
                        name = "description_html",
                        param_type = "string",
                        required = False,
                        description = "HTML description of the issue",
                    ),
                    ToolParameter(
                        name = "priority",
                        param_type = "string",
                        required = False,
                        default = "none",
                        description = "Priority level (urgent, high, medium, low, none)",
                    ),
                    ToolParameter(
                        name = "state_id",
                        param_type = "string",
                        required = False,
                        description = "ID of the initial state",
                    ),
                    ToolParameter(
                        name = "assignees",
                        param_type = "string",
                        required = False,
                        description = "Comma-separated list of assignee IDs",
                    ),
                ],
                handler = create_issue,
            ),
            Tool(
                name = "plane_list_issues",
                description = "List issues in a project with optional filters",
                parameters = [
                    ToolParameter(
                        name = "project_id",
                        param_type = "string",
                        required = True,
                        description = "The ID of the project",
                    ),
                    ToolParameter(
                        name = "state_id",
                        param_type = "string",
                        required = False,
                        description = "Filter by state ID",
                    ),
                    ToolParameter(
                        name = "priority",
                        param_type = "string",
                        required = False,
                        description = "Filter by priority",
                    ),
                    ToolParameter(
                        name = "assignee_id",
                        param_type = "string",
                        required = False,
                        description = "Filter by assignee ID",
                    ),
                    ToolParameter(
                        name = "limit",
                        param_type = "integer",
                        required = False,
                        default = "20",
                        description = "Maximum number of issues to return",
                    ),
                ],
                handler = list_issues,
            ),
            Tool(
                name = "plane_get_issue",
                description = "Get detailed information about a specific issue",
                parameters = [
                    ToolParameter(
                        name = "project_id",
                        param_type = "string",
                        required = True,
                        description = "The ID of the project",
                    ),
                    ToolParameter(
                        name = "issue_id",
                        param_type = "string",
                        required = True,
                        description = "The ID of the issue",
                    ),
                ],
                handler = get_issue,
            ),
            Tool(
                name = "plane_update_issue",
                description = "Update an existing issue",
                parameters = [
                    ToolParameter(
                        name = "project_id",
                        param_type = "string",
                        required = True,
                        description = "The ID of the project",
                    ),
                    ToolParameter(
                        name = "issue_id",
                        param_type = "string",
                        required = True,
                        description = "The ID of the issue to update",
                    ),
                    ToolParameter(
                        name = "name",
                        param_type = "string",
                        required = False,
                        description = "New name/title for the issue",
                    ),
                    ToolParameter(
                        name = "description_html",
                        param_type = "string",
                        required = False,
                        description = "New HTML description",
                    ),
                    ToolParameter(
                        name = "priority",
                        param_type = "string",
                        required = False,
                        description = "New priority level",
                    ),
                    ToolParameter(
                        name = "state_id",
                        param_type = "string",
                        required = False,
                        description = "New state ID",
                    ),
                    ToolParameter(
                        name = "assignees",
                        param_type = "string",
                        required = False,
                        description = "New comma-separated list of assignee IDs",
                    ),
                ],
                handler = update_issue,
            ),
        ],
    )
