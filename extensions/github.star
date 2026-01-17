# GitHub MCP Server Extension
# Provides GitHub integration tools via gh CLI

# Configuration helper
def get_github_config():
    """Get GitHub configuration from environment or defaults"""
    return {
        "default_repo": env.get("GITHUB_DEFAULT_REPO", ""),
    }

# Helper function to run gh command
def run_gh_command(args):
    """Execute a gh CLI command and return the result"""
    result = exec.run("gh", args)

    if not result["success"]:
        return {"error": result["stderr"], "success": False}

    return {"output": result["stdout"], "success": True}

# Tool implementations
def get_pr_review_comments(params):
    """Fetch review comments for a pull request"""
    repo = params.get("repo", "")
    pr_number = params.get("pr_number", "")
    user_filter = params.get("user", "")

    # Use default repo if not provided
    if not repo:
        config = get_github_config()
        repo = config["default_repo"]

    if not repo:
        return error_response("repo parameter is required (or set GITHUB_DEFAULT_REPO)")

    if not pr_number:
        return error_response("pr_number parameter is required")

    # Build gh command to get PR review comments
    # Using gh api to get review comments with full details
    args = [
        "api",
        "/repos/{}/pulls/{}/comments".format(repo, pr_number),
        "--jq",
        ".",
    ]

    result = run_gh_command(args)

    if not result["success"]:
        return error_response("Failed to fetch PR comments: " + result.get("error", "Unknown error"))

    # Parse JSON output
    comments_json = result["output"].strip()
    if not comments_json or comments_json == "[]":
        return {
            "content": [{"type": "text", "text": "No review comments found for PR #{}".format(pr_number)}],
            "structuredContent": {"repo": repo, "prNumber": pr_number, "comments": []},
        }

    comments = json.decode(comments_json)

    # Filter by user if specified
    if user_filter:
        filtered_comments = []
        for comment in comments:
            if comment.get("user", {}).get("login", "") == user_filter:
                filtered_comments.append(comment)
        comments = filtered_comments

    if not comments:
        filter_msg = " from user '{}'".format(user_filter) if user_filter else ""
        return {
            "content": [{"type": "text", "text": "No review comments found for PR #{}{}".format(pr_number, filter_msg)}],
            "structuredContent": {"repo": repo, "prNumber": pr_number, "userFilter": user_filter, "comments": []},
        }

    # Build structured data
    structured_comments = []
    for comment in comments:
        structured_comments.append({
            "id": comment.get("id"),
            "author": comment.get("user", {}).get("login", "unknown"),
            "path": comment.get("path", ""),
            "line": comment.get("line"),
            "body": comment.get("body", ""),
            "createdAt": comment.get("created_at", ""),
            "updatedAt": comment.get("updated_at", ""),
        })

    structured = {
        "repo": repo,
        "prNumber": pr_number,
        "userFilter": user_filter if user_filter else None,
        "commentCount": len(comments),
        "comments": structured_comments,
    }

    # Format the output
    output = "Review Comments for PR #{} in {}:\n".format(pr_number, repo)
    if user_filter:
        output += "Filtered by user: {}\n".format(user_filter)
    output += "=" * 60 + "\n\n"
    output += "Found {} comment(s)\n\n".format(len(comments))

    for i, comment in enumerate(comments, 1):
        user = comment.get("user", {}).get("login", "unknown")
        path = comment.get("path", "unknown")
        line = comment.get("line", "?")
        body = comment.get("body", "")
        created_at = comment.get("created_at", "")

        output += "Comment #{}\n".format(i)
        output += "-" * 40 + "\n"
        output += "Author: {}\n".format(user)
        output += "File: {} (line {})\n".format(path, line)
        output += "Date: {}\n".format(created_at)
        output += "\n{}\n\n".format(body)

    return {
        "content": [{"type": "text", "text": output}],
        "structuredContent": structured,
    }

def list_pull_requests(params):
    """List pull requests in a repository"""
    repo = params.get("repo", "")
    state = params.get("state", "open")
    limit = params.get("limit", "10")

    # Use default repo if not provided
    if not repo:
        config = get_github_config()
        repo = config["default_repo"]

    if not repo:
        return error_response("repo parameter is required (or set GITHUB_DEFAULT_REPO)")

    # Build gh command
    args = [
        "pr",
        "list",
        "--repo",
        repo,
        "--state",
        state,
        "--limit",
        str(limit),
        "--json",
        "number,title,author,state,createdAt,updatedAt",
    ]

    result = run_gh_command(args)

    if not result["success"]:
        return error_response("Failed to list PRs: " + result.get("error", "Unknown error"))

    # Parse JSON output
    prs_json = result["output"].strip()
    if not prs_json or prs_json == "[]":
        return {
            "content": [{"type": "text", "text": "No pull requests found"}],
            "structuredContent": {"repo": repo, "state": state, "pullRequests": []},
        }

    prs = json.decode(prs_json)

    # Build structured data
    structured_prs = []
    for pr in prs:
        structured_prs.append({
            "number": pr.get("number"),
            "title": pr.get("title", ""),
            "author": pr.get("author", {}).get("login", "unknown"),
            "state": pr.get("state", "unknown"),
            "createdAt": pr.get("createdAt", ""),
            "updatedAt": pr.get("updatedAt", ""),
        })

    structured = {
        "repo": repo,
        "state": state,
        "count": len(prs),
        "pullRequests": structured_prs,
    }

    # Format the output
    output = "Pull Requests in {} ({})\n".format(repo, state)
    output += "=" * 60 + "\n\n"
    output += "Found {} PR(s)\n\n".format(len(prs))

    for pr in prs:
        number = pr.get("number", "?")
        title = pr.get("title", "")
        author = pr.get("author", {}).get("login", "unknown")
        state_val = pr.get("state", "unknown")
        created = pr.get("createdAt", "")

        output += "#{} - {}\n".format(number, title)
        output += "  Author: {} | State: {} | Created: {}\n\n".format(author, state_val, created)

    return {
        "content": [{"type": "text", "text": output}],
        "structuredContent": structured,
    }

def get_pr_details(params):
    """Get detailed information about a specific pull request"""
    repo = params.get("repo", "")
    pr_number = params.get("pr_number", "")

    # Use default repo if not provided
    if not repo:
        config = get_github_config()
        repo = config["default_repo"]

    if not repo:
        return error_response("repo parameter is required (or set GITHUB_DEFAULT_REPO)")

    if not pr_number:
        return error_response("pr_number parameter is required")

    # Build gh command
    args = [
        "pr",
        "view",
        str(pr_number),
        "--repo",
        repo,
        "--json",
        "number,title,body,author,state,createdAt,updatedAt,mergeable,reviewDecision,additions,deletions,changedFiles",
    ]

    result = run_gh_command(args)

    if not result["success"]:
        return error_response("Failed to get PR details: " + result.get("error", "Unknown error"))

    # Parse JSON output
    pr = json.decode(result["output"].strip())

    # Build structured data
    structured = {
        "repo": repo,
        "number": pr.get("number"),
        "title": pr.get("title", ""),
        "body": pr.get("body", ""),
        "author": pr.get("author", {}).get("login", "unknown"),
        "state": pr.get("state", "unknown"),
        "reviewDecision": pr.get("reviewDecision"),
        "mergeable": pr.get("mergeable"),
        "createdAt": pr.get("createdAt", ""),
        "updatedAt": pr.get("updatedAt", ""),
        "changes": {
            "additions": pr.get("additions", 0),
            "deletions": pr.get("deletions", 0),
            "changedFiles": pr.get("changedFiles", 0),
        },
    }

    # Format the output
    output = "Pull Request #{} in {}\n".format(pr.get("number", "?"), repo)
    output += "=" * 60 + "\n\n"
    output += "Title: {}\n".format(pr.get("title", ""))
    output += "Author: {}\n".format(pr.get("author", {}).get("login", "unknown"))
    output += "State: {}\n".format(pr.get("state", "unknown"))
    output += "Review Decision: {}\n".format(pr.get("reviewDecision", "none"))
    output += "Mergeable: {}\n".format(pr.get("mergeable", "unknown"))
    output += "Created: {}\n".format(pr.get("createdAt", ""))
    output += "Updated: {}\n".format(pr.get("updatedAt", ""))
    output += "\nChanges:\n"
    output += "  +{} additions, -{} deletions\n".format(
        pr.get("additions", 0),
        pr.get("deletions", 0),
    )
    output += "  {} files changed\n".format(pr.get("changedFiles", 0))

    body = pr.get("body", "")
    if body:
        output += "\nDescription:\n"
        output += "-" * 40 + "\n"
        output += "{}\n".format(body)

    return {
        "content": [{"type": "text", "text": output}],
        "structuredContent": structured,
    }

def get_pr_reviews(params):
    """Get reviews for a pull request"""
    repo = params.get("repo", "")
    pr_number = params.get("pr_number", "")

    # Use default repo if not provided
    if not repo:
        config = get_github_config()
        repo = config["default_repo"]

    if not repo:
        return error_response("repo parameter is required (or set GITHUB_DEFAULT_REPO)")

    if not pr_number:
        return error_response("pr_number parameter is required")

    # Build gh command to get reviews
    args = [
        "api",
        "/repos/{}/pulls/{}/reviews".format(repo, pr_number),
        "--jq",
        ".",
    ]

    result = run_gh_command(args)

    if not result["success"]:
        return error_response("Failed to fetch PR reviews: " + result.get("error", "Unknown error"))

    # Parse JSON output
    reviews_json = result["output"].strip()
    if not reviews_json or reviews_json == "[]":
        return {
            "content": [{"type": "text", "text": "No reviews found for PR #{}".format(pr_number)}],
            "structuredContent": {"repo": repo, "prNumber": pr_number, "reviews": []},
        }

    reviews = json.decode(reviews_json)

    # Build structured data
    structured_reviews = []
    for review in reviews:
        structured_reviews.append({
            "id": review.get("id"),
            "reviewer": review.get("user", {}).get("login", "unknown"),
            "state": review.get("state", "unknown"),
            "body": review.get("body", ""),
            "submittedAt": review.get("submitted_at", ""),
        })

    structured = {
        "repo": repo,
        "prNumber": pr_number,
        "reviewCount": len(reviews),
        "reviews": structured_reviews,
    }

    # Format the output
    output = "Reviews for PR #{} in {}:\n".format(pr_number, repo)
    output += "=" * 60 + "\n\n"
    output += "Found {} review(s)\n\n".format(len(reviews))

    for i, review in enumerate(reviews, 1):
        user = review.get("user", {}).get("login", "unknown")
        state = review.get("state", "unknown")
        body = review.get("body", "")
        submitted_at = review.get("submitted_at", "")

        # Map state to emoji
        state_emoji = {
            "APPROVED": "✅",
            "CHANGES_REQUESTED": "❌",
            "COMMENTED": "💬",
        }.get(state, "❓")

        output += "Review #{} {}\n".format(i, state_emoji)
        output += "-" * 40 + "\n"
        output += "Reviewer: {}\n".format(user)
        output += "State: {}\n".format(state)
        output += "Date: {}\n".format(submitted_at)

        if body:
            output += "\n{}\n".format(body)
        output += "\n"

    return {
        "content": [{"type": "text", "text": output}],
        "structuredContent": structured,
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
    """Define the GitHub MCP extension"""
    return Extension(
        name = "github",
        version = "1.0.0",
        description = "GitHub integration via gh CLI",
        allowed_exec = ["gh"],
        tools = [
            Tool(
                name = "github_pr_review_comments",
                description = "Fetch review comments for a pull request with optional user filter",
                parameters = [
                    ToolParameter(
                        name = "repo",
                        param_type = "string",
                        required = False,
                        description = "Repository in format 'owner/repo' (uses GITHUB_DEFAULT_REPO if not provided)",
                    ),
                    ToolParameter(
                        name = "pr_number",
                        param_type = "string",
                        required = True,
                        description = "Pull request number",
                    ),
                    ToolParameter(
                        name = "user",
                        param_type = "string",
                        required = False,
                        description = "Filter comments by GitHub username",
                    ),
                ],
                handler = get_pr_review_comments,
            ),
            Tool(
                name = "github_list_prs",
                description = "List pull requests in a repository",
                parameters = [
                    ToolParameter(
                        name = "repo",
                        param_type = "string",
                        required = False,
                        description = "Repository in format 'owner/repo' (uses GITHUB_DEFAULT_REPO if not provided)",
                    ),
                    ToolParameter(
                        name = "state",
                        param_type = "string",
                        required = False,
                        default = "open",
                        description = "PR state: open, closed, merged, or all",
                    ),
                    ToolParameter(
                        name = "limit",
                        param_type = "string",
                        required = False,
                        default = "10",
                        description = "Maximum number of PRs to return",
                    ),
                ],
                handler = list_pull_requests,
            ),
            Tool(
                name = "github_pr_details",
                description = "Get detailed information about a specific pull request",
                parameters = [
                    ToolParameter(
                        name = "repo",
                        param_type = "string",
                        required = False,
                        description = "Repository in format 'owner/repo' (uses GITHUB_DEFAULT_REPO if not provided)",
                    ),
                    ToolParameter(
                        name = "pr_number",
                        param_type = "string",
                        required = True,
                        description = "Pull request number",
                    ),
                ],
                handler = get_pr_details,
            ),
            Tool(
                name = "github_pr_reviews",
                description = "Get all reviews for a pull request",
                parameters = [
                    ToolParameter(
                        name = "repo",
                        param_type = "string",
                        required = False,
                        description = "Repository in format 'owner/repo' (uses GITHUB_DEFAULT_REPO if not provided)",
                    ),
                    ToolParameter(
                        name = "pr_number",
                        param_type = "string",
                        required = True,
                        description = "Pull request number",
                    ),
                ],
                handler = get_pr_reviews,
            ),
        ],
    )
