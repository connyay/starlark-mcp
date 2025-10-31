# Docker MCP Extension
# Provides tools to inspect Docker containers, images, volumes, and system information

def list_containers(params):
    """List all Docker containers (running and stopped)"""
    show_all = params.get("all", True)

    args = ["ps", "--format", "json"]
    if show_all:
        args.append("--all")

    result = exec.run("docker", args)

    if not result["success"]:
        return error_response("Failed to list containers: " + result["stderr"])

    # Parse JSON lines output
    containers = []
    stdout = result["stdout"].strip()
    if stdout:
        lines = stdout.split("\n")
        for line in lines:
            if line.strip():
                container = json.decode(line)
                containers.append(container)

    if not containers:
        return {
            "content": [{"type": "text", "text": "No containers found"}],
        }

    # Format output
    output = "Found {} container(s):\n\n".format(len(containers))
    for c in containers:
        status_icon = "🟢" if "Up" in c.get("Status", "") else "🔴"
        output += "{} {} ({})\n".format(
            status_icon,
            c.get("Names", "unknown"),
            c.get("ID", "")[:12],
        )
        output += "  Image: {}\n".format(c.get("Image", ""))
        output += "  Status: {}\n".format(c.get("Status", ""))
        output += "  Ports: {}\n".format(c.get("Ports", ""))
        output += "\n"

    return {
        "content": [{"type": "text", "text": output}],
    }

def inspect_container(params):
    """Get detailed information about a specific container"""
    container_id = params.get("container_id", "")

    if not container_id:
        return error_response("container_id parameter is required")

    result = exec.run("docker", ["inspect", container_id])

    if not result["success"]:
        return error_response("Failed to inspect container: " + result["stderr"])

    # Parse JSON output
    inspect_data = json.decode(result["stdout"])
    if not inspect_data or len(inspect_data) == 0:
        return error_response("Container not found: " + container_id)

    container = inspect_data[0]

    # Format detailed output
    output = "Container: {}\n".format(container.get("Name", "").lstrip("/"))
    output += "=" * 50 + "\n\n"

    output += "ID: {}\n".format(container.get("Id", "")[:12])
    output += "Image: {}\n".format(container.get("Config", {}).get("Image", ""))
    output += "Status: {}\n".format(container.get("State", {}).get("Status", ""))

    state = container.get("State", {})
    if state.get("Running"):
        output += "Running: Yes\n"
        output += "Started At: {}\n".format(state.get("StartedAt", ""))
    else:
        output += "Running: No\n"
        if state.get("FinishedAt"):
            output += "Finished At: {}\n".format(state.get("FinishedAt", ""))
        if state.get("ExitCode"):
            output += "Exit Code: {}\n".format(state.get("ExitCode", ""))

    # Network info
    networks = container.get("NetworkSettings", {}).get("Networks", {})
    if networks:
        output += "\nNetworks:\n"
        for name, net in networks.items():
            output += "  {}: {}\n".format(name, net.get("IPAddress", ""))

    # Mounts
    mounts = container.get("Mounts", [])
    if mounts:
        output += "\nMounts:\n"
        for mount in mounts:
            output += "  {} -> {} ({})\n".format(
                mount.get("Source", ""),
                mount.get("Destination", ""),
                mount.get("Type", ""),
            )

    # Environment variables
    env = container.get("Config", {}).get("Env", [])
    if env:
        output += "\nEnvironment Variables:\n"
        for e in env[:10]:  # Limit to first 10
            output += "  {}\n".format(e)
        if len(env) > 10:
            output += "  ... and {} more\n".format(len(env) - 10)

    return {
        "content": [{"type": "text", "text": output}],
    }

def container_logs(params):
    """Get logs from a container"""
    container_id = params.get("container_id", "")
    tail = params.get("tail", "100")

    if not container_id:
        return error_response("container_id parameter is required")

    result = exec.run("docker", ["logs", "--tail", str(tail), container_id])

    if not result["success"]:
        return error_response("Failed to get logs: " + result["stderr"])

    output = "Logs from container {} (last {} lines):\n\n".format(container_id, tail)
    output += result["stdout"]

    if result["stderr"]:
        output += "\n\nStderr:\n" + result["stderr"]

    return {
        "content": [{"type": "text", "text": output}],
    }

def list_images(params):
    """List all Docker images"""
    result = exec.run("docker", ["images", "--format", "json"])

    if not result["success"]:
        return error_response("Failed to list images: " + result["stderr"])

    # Parse JSON lines output
    images = []
    stdout = result["stdout"].strip()
    if stdout:
        lines = stdout.split("\n")
        for line in lines:
            if line.strip():
                image = json.decode(line)
                images.append(image)

    if not images:
        return {
            "content": [{"type": "text", "text": "No images found"}],
        }

    # Format output
    output = "Found {} image(s):\n\n".format(len(images))
    for img in images:
        output += "📦 {}:{}\n".format(
            img.get("Repository", ""),
            img.get("Tag", ""),
        )
        output += "  ID: {}\n".format(img.get("ID", ""))
        output += "  Size: {}\n".format(img.get("Size", ""))
        output += "  Created: {}\n".format(img.get("CreatedSince", ""))
        output += "\n"

    return {
        "content": [{"type": "text", "text": output}],
    }

def list_volumes(params):
    """List all Docker volumes"""
    result = exec.run("docker", ["volume", "ls", "--format", "json"])

    if not result["success"]:
        return error_response("Failed to list volumes: " + result["stderr"])

    # Parse JSON lines output
    volumes = []
    stdout = result["stdout"].strip()
    if stdout:
        lines = stdout.split("\n")
        for line in lines:
            if line.strip():
                volume = json.decode(line)
                volumes.append(volume)

    if not volumes:
        return {
            "content": [{"type": "text", "text": "No volumes found"}],
        }

    # Format output
    output = "Found {} volume(s):\n\n".format(len(volumes))
    for vol in volumes:
        output += "💾 {}\n".format(vol.get("Name", ""))
        output += "  Driver: {}\n".format(vol.get("Driver", ""))
        if vol.get("Mountpoint"):
            output += "  Mountpoint: {}\n".format(vol.get("Mountpoint", ""))
        output += "\n"

    return {
        "content": [{"type": "text", "text": output}],
    }

def inspect_volume(params):
    """Get detailed information about a specific volume"""
    volume_name = params.get("volume_name", "")

    if not volume_name:
        return error_response("volume_name parameter is required")

    result = exec.run("docker", ["volume", "inspect", volume_name])

    if not result["success"]:
        return error_response("Failed to inspect volume: " + result["stderr"])

    # Parse JSON output
    inspect_data = json.decode(result["stdout"])
    if not inspect_data or len(inspect_data) == 0:
        return error_response("Volume not found: " + volume_name)

    volume = inspect_data[0]

    # Format output
    output = "Volume: {}\n".format(volume.get("Name", ""))
    output += "=" * 50 + "\n\n"
    output += "Driver: {}\n".format(volume.get("Driver", ""))
    output += "Mountpoint: {}\n".format(volume.get("Mountpoint", ""))
    output += "Created: {}\n".format(volume.get("CreatedAt", ""))

    # Labels
    labels = volume.get("Labels", {})
    if labels:
        output += "\nLabels:\n"
        for key, value in labels.items():
            output += "  {}: {}\n".format(key, value)

    # Options
    options = volume.get("Options", {})
    if options:
        output += "\nOptions:\n"
        for key, value in options.items():
            output += "  {}: {}\n".format(key, value)

    return {
        "content": [{"type": "text", "text": output}],
    }

def system_info(params):
    """Get Docker system information and disk usage"""

    # Get system info
    info_result = exec.run("docker", ["info", "--format", "json"])
    if not info_result["success"]:
        return error_response("Failed to get system info: " + info_result["stderr"])

    # Get disk usage
    df_result = exec.run("docker", ["system", "df", "--format", "json"])
    if not df_result["success"]:
        return error_response("Failed to get disk usage: " + df_result["stderr"])

    info = json.decode(info_result["stdout"])

    output = "Docker System Information\n"
    output += "=" * 50 + "\n\n"

    output += "Server Version: {}\n".format(info.get("ServerVersion", ""))
    output += "Storage Driver: {}\n".format(info.get("Driver", ""))
    output += "Operating System: {}\n".format(info.get("OperatingSystem", ""))
    output += "OS Type: {}\n".format(info.get("OSType", ""))
    output += "Architecture: {}\n".format(info.get("Architecture", ""))
    output += "CPUs: {}\n".format(info.get("NCPU", ""))
    mem_gb = info.get("MemTotal", 0) / 1073741824
    output += "Total Memory: {} GB\n".format(int(mem_gb) if mem_gb > 0 else 0)

    output += "\nContainer Stats:\n"
    output += "  Running: {}\n".format(info.get("ContainersRunning", 0))
    output += "  Paused: {}\n".format(info.get("ContainersPaused", 0))
    output += "  Stopped: {}\n".format(info.get("ContainersStopped", 0))
    output += "  Total: {}\n".format(info.get("Containers", 0))

    output += "\nImage Count: {}\n".format(info.get("Images", 0))

    # Parse disk usage
    df_lines = df_result["stdout"].strip().split("\n")
    if df_lines:
        output += "\nDisk Usage:\n"
        for line in df_lines:
            if line.strip():
                item = json.decode(line)
                item_type = item.get("Type", "")
                output += "  {}: {} items, {} active, {} (reclaimable: {})\n".format(
                    item_type,
                    item.get("TotalCount", 0),
                    item.get("Active", 0),
                    item.get("Size", ""),
                    item.get("Reclaimable", ""),
                )

    return {
        "content": [{"type": "text", "text": output}],
    }

def container_stats(params):
    """Get resource usage statistics for containers"""
    container_id = params.get("container_id", "")

    args = ["stats", "--no-stream", "--format", "json"]
    if container_id:
        args.append(container_id)
    else:
        args.append("--all")

    result = exec.run("docker", args)

    if not result["success"]:
        return error_response("Failed to get stats: " + result["stderr"])

    # Parse JSON lines output
    stats = []
    stdout = result["stdout"].strip()
    if stdout:
        lines = stdout.split("\n")
        for line in lines:
            if line.strip():
                stat = json.decode(line)
                stats.append(stat)

    if not stats:
        return {
            "content": [{"type": "text", "text": "No statistics available"}],
        }

    # Format output
    output = "Container Resource Usage:\n\n"
    for s in stats:
        output += "📊 {} ({})\n".format(
            s.get("Name", ""),
            s.get("ID", "")[:12],
        )
        output += "  CPU: {}\n".format(s.get("CPUPerc", ""))
        output += "  Memory: {} / {}\n".format(
            s.get("MemUsage", "").split(" / ")[0] if " / " in s.get("MemUsage", "") else "",
            s.get("MemUsage", "").split(" / ")[1] if " / " in s.get("MemUsage", "") else "",
        )
        output += "  Memory %: {}\n".format(s.get("MemPerc", ""))
        output += "  Net I/O: {}\n".format(s.get("NetIO", ""))
        output += "  Block I/O: {}\n".format(s.get("BlockIO", ""))
        output += "\n"

    return {
        "content": [{"type": "text", "text": output}],
    }

def list_networks(params):
    """List all Docker networks"""
    result = exec.run("docker", ["network", "ls", "--format", "json"])

    if not result["success"]:
        return error_response("Failed to list networks: " + result["stderr"])

    # Parse JSON lines output
    networks = []
    stdout = result["stdout"].strip()
    if stdout:
        lines = stdout.split("\n")
        for line in lines:
            if line.strip():
                network = json.decode(line)
                networks.append(network)

    if not networks:
        return {
            "content": [{"type": "text", "text": "No networks found"}],
        }

    # Format output
    output = "Found {} network(s):\n\n".format(len(networks))
    for net in networks:
        output += "🌐 {}\n".format(net.get("Name", ""))
        output += "  ID: {}\n".format(net.get("ID", "")[:12])
        output += "  Driver: {}\n".format(net.get("Driver", ""))
        output += "  Scope: {}\n".format(net.get("Scope", ""))
        output += "\n"

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
    """Define the Docker MCP extension"""
    return Extension(
        name = "docker",
        version = "1.0.0",
        description = "Docker container, image, and system inspection tools",
        allowed_exec = ["docker"],
        tools = [
            Tool(
                name = "docker_list_containers",
                description = "List all Docker containers (running and stopped)",
                parameters = [
                    ToolParameter(
                        name = "all",
                        param_type = "boolean",
                        required = False,
                        default = "true",
                        description = "Show all containers (default: true). Set to false to show only running containers",
                    ),
                ],
                handler = list_containers,
            ),
            Tool(
                name = "docker_inspect_container",
                description = "Get detailed information about a specific container",
                parameters = [
                    ToolParameter(
                        name = "container_id",
                        param_type = "string",
                        required = True,
                        description = "Container ID or name",
                    ),
                ],
                handler = inspect_container,
            ),
            Tool(
                name = "docker_container_logs",
                description = "Get logs from a container",
                parameters = [
                    ToolParameter(
                        name = "container_id",
                        param_type = "string",
                        required = True,
                        description = "Container ID or name",
                    ),
                    ToolParameter(
                        name = "tail",
                        param_type = "integer",
                        required = False,
                        default = "100",
                        description = "Number of lines to show from the end of the logs (default: 100)",
                    ),
                ],
                handler = container_logs,
            ),
            Tool(
                name = "docker_list_images",
                description = "List all Docker images",
                parameters = [],
                handler = list_images,
            ),
            Tool(
                name = "docker_list_volumes",
                description = "List all Docker volumes",
                parameters = [],
                handler = list_volumes,
            ),
            Tool(
                name = "docker_inspect_volume",
                description = "Get detailed information about a specific volume",
                parameters = [
                    ToolParameter(
                        name = "volume_name",
                        param_type = "string",
                        required = True,
                        description = "Volume name",
                    ),
                ],
                handler = inspect_volume,
            ),
            Tool(
                name = "docker_system_info",
                description = "Get Docker system information and disk usage statistics",
                parameters = [],
                handler = system_info,
            ),
            Tool(
                name = "docker_container_stats",
                description = "Get real-time resource usage statistics for containers (CPU, memory, network, disk I/O)",
                parameters = [
                    ToolParameter(
                        name = "container_id",
                        param_type = "string",
                        required = False,
                        description = "Container ID or name. If not provided, shows stats for all running containers",
                    ),
                ],
                handler = container_stats,
            ),
            Tool(
                name = "docker_list_networks",
                description = "List all Docker networks",
                parameters = [],
                handler = list_networks,
            ),
        ],
    )
