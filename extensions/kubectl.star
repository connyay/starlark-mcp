# Kubernetes kubectl Extension
# Provides tools for gathering Kubernetes cluster data via kubectl

# Configuration helpers
def build_kubectl_args(subcommand_args):
    """Build kubectl arguments array with optional kubeconfig and context"""
    args = []

    # Check for custom kubeconfig
    kubeconfig = env.get("KUBECONFIG", "")
    if kubeconfig:
        args.append("--kubeconfig=" + kubeconfig)

    # Check for custom context
    context = env.get("K8S_CONTEXT", "")
    if context:
        args.append("--context=" + context)

    # Add the actual command arguments
    return args + subcommand_args

def get_kubectl_namespace():
    """Get default namespace from environment"""
    return env.get("K8S_NAMESPACE", "default")

# Tool implementations
def get_cluster_info(params):
    """Get basic cluster information"""

    # Get cluster info
    args = build_kubectl_args(["cluster-info"])
    result = exec.run("kubectl", args)

    if not result["success"]:
        return error_response("Failed to get cluster info: " + result.get("stderr", ""))

    output = "Cluster Information:\n"
    output += "=" * 50 + "\n\n"
    output += result["stdout"]

    # Get version info
    version_args = build_kubectl_args(["version"])
    version_result = exec.run("kubectl", version_args)
    if version_result["success"]:
        output += "\n\nVersion Information:\n"
        output += "-" * 50 + "\n"
        output += version_result["stdout"]

    return {"content": [{"type": "text", "text": output}]}

def list_namespaces(params):
    """List all namespaces in the cluster"""
    args = build_kubectl_args(["get", "namespaces", "-o", "json"])
    result = exec.run("kubectl", args)

    if not result["success"]:
        return error_response("Failed to list namespaces: " + result.get("stderr", ""))

    data = json.decode(result["stdout"])
    items = data.get("items", [])

    output = "Found {} namespace(s):\n\n".format(len(items))

    for ns in items:
        name = ns.get("metadata", {}).get("name", "")
        status = ns.get("status", {}).get("phase", "")
        created = ns.get("metadata", {}).get("creationTimestamp", "")

        output += "📦 {} ({})\n".format(name, status)
        output += "   Created: {}\n\n".format(created)

    return {"content": [{"type": "text", "text": output}]}

def list_pods(params):
    """List pods in a namespace"""
    namespace = params.get("namespace", get_kubectl_namespace())
    all_namespaces = params.get("all_namespaces", False)

    if all_namespaces:
        args = build_kubectl_args(["get", "pods", "--all-namespaces", "-o", "json"])
    else:
        args = build_kubectl_args(["get", "pods", "-n", namespace, "-o", "json"])

    result = exec.run("kubectl", args)

    if not result["success"]:
        return error_response("Failed to list pods: " + result.get("stderr", ""))

    data = json.decode(result["stdout"])
    items = data.get("items", [])

    output = "Found {} pod(s)".format(len(items))
    if not all_namespaces:
        output += " in namespace '{}'".format(namespace)
    output += ":\n\n"

    for pod in items:
        metadata = pod.get("metadata", {})
        spec = pod.get("spec", {})
        status = pod.get("status", {})

        name = metadata.get("name", "")
        ns = metadata.get("namespace", "")
        phase = status.get("phase", "")

        # Count containers
        containers = spec.get("containers", [])
        container_statuses = status.get("containerStatuses", [])
        ready_count = 0
        for cs in container_statuses:
            if cs.get("ready", False):
                ready_count += 1

        if all_namespaces:
            output += "🎯 {}/{} ({}) - {}/{} ready\n".format(
                ns,
                name,
                phase,
                ready_count,
                len(containers),
            )
        else:
            output += "🎯 {} ({}) - {}/{} ready\n".format(
                name,
                phase,
                ready_count,
                len(containers),
            )

        # Show node
        node = spec.get("nodeName", "")
        if node:
            output += "   Node: {}\n".format(node)

        # Show restart count
        total_restarts = 0
        for cs in container_statuses:
            total_restarts += cs.get("restartCount", 0)
        if total_restarts > 0:
            output += "   Restarts: {}\n".format(total_restarts)

        output += "\n"

    return {"content": [{"type": "text", "text": output}]}

def get_pod_details(params):
    """Get detailed information about a specific pod"""
    pod_name = params.get("pod", "")
    namespace = params.get("namespace", get_kubectl_namespace())

    if not pod_name:
        return error_response("pod parameter is required")

    # Get pod details
    args = build_kubectl_args(["get", "pod", pod_name, "-n", namespace, "-o", "json"])
    result = exec.run("kubectl", args)

    if not result["success"]:
        return error_response("Failed to get pod details: " + result.get("stderr", ""))

    pod = json.decode(result["stdout"])
    metadata = pod.get("metadata", {})
    spec = pod.get("spec", {})
    status = pod.get("status", {})

    output = "Pod: {}/{}\n".format(namespace, metadata.get("name", ""))
    output += "=" * 50 + "\n\n"

    # Basic info
    output += "Status: {}\n".format(status.get("phase", ""))
    output += "Node: {}\n".format(spec.get("nodeName", ""))
    output += "Start Time: {}\n".format(status.get("startTime", ""))
    output += "IP: {}\n".format(status.get("podIP", ""))

    # Labels
    labels = metadata.get("labels", {})
    if labels:
        output += "\nLabels:\n"
        for key, value in labels.items():
            output += "  {}: {}\n".format(key, value)

    # Containers
    output += "\nContainers:\n"
    containers = spec.get("containers", [])
    container_statuses = status.get("containerStatuses", [])

    for i, container in enumerate(containers):
        output += "  - {}\n".format(container.get("name", ""))
        output += "    Image: {}\n".format(container.get("image", ""))

        # Find matching status
        for cs in container_statuses:
            if cs.get("name", "") == container.get("name", ""):
                output += "    Ready: {}\n".format(cs.get("ready", False))
                output += "    Restarts: {}\n".format(cs.get("restartCount", 0))
                state = cs.get("state", {})
                for state_type, state_info in state.items():
                    output += "    State: {}\n".format(state_type)
                break

    # Conditions
    conditions = status.get("conditions", [])
    if conditions:
        output += "\nConditions:\n"
        for condition in conditions:
            ctype = condition.get("type", "")
            cstatus = condition.get("status", "")
            output += "  {} = {}\n".format(ctype, cstatus)

    return {"content": [{"type": "text", "text": output}]}

def get_pod_logs(params):
    """Get logs from a pod"""
    pod_name = params.get("pod", "")
    namespace = params.get("namespace", get_kubectl_namespace())
    container = params.get("container", "")
    tail = params.get("tail", "100")
    previous = params.get("previous", False)

    if not pod_name:
        return error_response("pod parameter is required")

    args = ["logs", pod_name, "-n", namespace]

    if container:
        args.extend(["-c", container])

    if tail:
        args.append("--tail=" + tail)

    if previous:
        args.append("--previous")

    kubectl_args = build_kubectl_args(args)
    result = exec.run("kubectl", kubectl_args)

    if not result["success"]:
        return error_response("Failed to get logs: " + result.get("stderr", ""))

    output = "Logs from {}/{}".format(namespace, pod_name)
    if container:
        output += " (container: {})".format(container)
    output += ":\n"
    output += "=" * 50 + "\n\n"
    output += result["stdout"]

    return {"content": [{"type": "text", "text": output}]}

def list_services(params):
    """List services in a namespace"""
    namespace = params.get("namespace", get_kubectl_namespace())
    all_namespaces = params.get("all_namespaces", False)

    if all_namespaces:
        args = build_kubectl_args(["get", "services", "--all-namespaces", "-o", "json"])
    else:
        args = build_kubectl_args(["get", "services", "-n", namespace, "-o", "json"])

    result = exec.run("kubectl", args)

    if not result["success"]:
        return error_response("Failed to list services: " + result.get("stderr", ""))

    data = json.decode(result["stdout"])
    items = data.get("items", [])

    output = "Found {} service(s)".format(len(items))
    if not all_namespaces:
        output += " in namespace '{}'".format(namespace)
    output += ":\n\n"

    for svc in items:
        metadata = svc.get("metadata", {})
        spec = svc.get("spec", {})

        name = metadata.get("name", "")
        ns = metadata.get("namespace", "")
        svc_type = spec.get("type", "")
        cluster_ip = spec.get("clusterIP", "")

        if all_namespaces:
            output += "🔗 {}/{} ({})\n".format(ns, name, svc_type)
        else:
            output += "🔗 {} ({})\n".format(name, svc_type)

        output += "   Cluster IP: {}\n".format(cluster_ip)

        # Show ports
        ports = spec.get("ports", [])
        if ports:
            port_strs = []
            for port in ports:
                port_str = "{}:{}".format(port.get("port", ""), port.get("targetPort", ""))
                protocol = port.get("protocol", "TCP")
                if protocol != "TCP":
                    port_str += "/{}".format(protocol)
                port_strs.append(port_str)
            output += "   Ports: {}\n".format(", ".join(port_strs))

        # Show external IP for LoadBalancer
        if svc_type == "LoadBalancer":
            external_ips = spec.get("externalIPs", [])
            if external_ips:
                output += "   External IPs: {}\n".format(", ".join(external_ips))

        output += "\n"

    return {"content": [{"type": "text", "text": output}]}

def list_deployments(params):
    """List deployments in a namespace"""
    namespace = params.get("namespace", get_kubectl_namespace())
    all_namespaces = params.get("all_namespaces", False)

    if all_namespaces:
        args = build_kubectl_args(["get", "deployments", "--all-namespaces", "-o", "json"])
    else:
        args = build_kubectl_args(["get", "deployments", "-n", namespace, "-o", "json"])

    result = exec.run("kubectl", args)

    if not result["success"]:
        return error_response("Failed to list deployments: " + result.get("stderr", ""))

    data = json.decode(result["stdout"])
    items = data.get("items", [])

    output = "Found {} deployment(s)".format(len(items))
    if not all_namespaces:
        output += " in namespace '{}'".format(namespace)
    output += ":\n\n"

    for deploy in items:
        metadata = deploy.get("metadata", {})
        spec = deploy.get("spec", {})
        status = deploy.get("status", {})

        name = metadata.get("name", "")
        ns = metadata.get("namespace", "")
        replicas = spec.get("replicas", 0)
        ready_replicas = status.get("readyReplicas", 0)
        available_replicas = status.get("availableReplicas", 0)

        if all_namespaces:
            output += "📊 {}/{} - {}/{} ready\n".format(
                ns,
                name,
                ready_replicas,
                replicas,
            )
        else:
            output += "📊 {} - {}/{} ready\n".format(
                name,
                ready_replicas,
                replicas,
            )

        # Show conditions
        conditions = status.get("conditions", [])
        for condition in conditions:
            if condition.get("type") == "Available":
                cstatus = condition.get("status", "")
                if cstatus != "True":
                    output += "   Status: Not Available - {}\n".format(
                        condition.get("message", ""),
                    )

        output += "\n"

    return {"content": [{"type": "text", "text": output}]}

def list_nodes(params):
    """List all nodes in the cluster"""
    args = build_kubectl_args(["get", "nodes", "-o", "json"])
    result = exec.run("kubectl", args)

    if not result["success"]:
        return error_response("Failed to list nodes: " + result.get("stderr", ""))

    data = json.decode(result["stdout"])
    items = data.get("items", [])

    output = "Found {} node(s):\n\n".format(len(items))

    for node in items:
        metadata = node.get("metadata", {})
        status = node.get("status", {})

        name = metadata.get("name", "")

        # Get node status
        conditions = status.get("conditions", [])
        node_ready = False
        for condition in conditions:
            if condition.get("type") == "Ready":
                node_ready = condition.get("status") == "True"
                break

        status_str = "Ready" if node_ready else "NotReady"

        # Get node info
        node_info = status.get("nodeInfo", {})
        os_image = node_info.get("osImage", "")
        kubelet_version = node_info.get("kubeletVersion", "")

        output += "🖥️  {} ({})\n".format(name, status_str)
        output += "   OS: {}\n".format(os_image)
        output += "   Kubelet: {}\n".format(kubelet_version)

        # Get addresses
        addresses = status.get("addresses", [])
        for addr in addresses:
            addr_type = addr.get("type", "")
            addr_val = addr.get("address", "")
            if addr_type in ["InternalIP", "ExternalIP"]:
                output += "   {}: {}\n".format(addr_type, addr_val)

        output += "\n"

    return {"content": [{"type": "text", "text": output}]}

def get_resource(params):
    """Get any Kubernetes resource by type and name"""
    resource_type = params.get("type", "")
    name = params.get("name", "")
    namespace = params.get("namespace", get_kubectl_namespace())
    output_format = params.get("format", "yaml")

    if not resource_type:
        return error_response("type parameter is required")

    # Build arguments
    args = ["get", resource_type]

    if name:
        args.append(name)

    # Add namespace if not a cluster-scoped resource
    cluster_resources = ["nodes", "namespaces", "persistentvolumes", "clusterroles", "clusterrolebindings"]
    if resource_type not in cluster_resources:
        args.extend(["-n", namespace])

    args.extend(["-o", output_format])

    kubectl_args = build_kubectl_args(args)
    result = exec.run("kubectl", kubectl_args)

    if not result["success"]:
        return error_response("Failed to get resource: " + result.get("stderr", ""))

    return {"content": [{"type": "text", "text": result["stdout"]}]}

def execute_kubectl(params):
    """Execute arbitrary kubectl command"""
    args_str = params.get("args", "")

    if not args_str:
        return error_response("args parameter is required")

    # Security: block dangerous commands
    dangerous = ["delete", "exec", "apply", "create", "patch", "replace", "edit"]
    args_lower = args_str.lower()
    for word in dangerous:
        if word in args_lower:
            return error_response("Command contains potentially dangerous operation: {}".format(word))

    # Split args string into list (simple split on spaces)
    # Note: This won't handle quoted arguments properly, but it's good enough for read-only commands
    args_list = args_str.split()
    kubectl_args = build_kubectl_args(args_list)
    result = exec.run("kubectl", kubectl_args)

    if not result["success"]:
        return error_response("Command failed: " + result.get("stderr", ""))

    return {"content": [{"type": "text", "text": result["stdout"]}]}

# Helper functions
def error_response(message):
    """Create an error response"""
    return {
        "content": [{"type": "text", "text": "Error: " + message}],
        "isError": True,
    }

# Extension definition
def describe_extension():
    """Define the kubectl extension"""
    return Extension(
        name = "kubectl",
        version = "1.0.0",
        description = "Kubernetes cluster data gathering via kubectl",
        allowed_exec = ["kubectl"],
        tools = [
            Tool(
                name = "k8s_cluster_info",
                description = "Get cluster information and version",
                parameters = [],
                handler = get_cluster_info,
            ),
            Tool(
                name = "k8s_list_namespaces",
                description = "List all namespaces in the cluster",
                parameters = [],
                handler = list_namespaces,
            ),
            Tool(
                name = "k8s_list_pods",
                description = "List pods in a namespace or across all namespaces",
                parameters = [
                    ToolParameter(
                        name = "namespace",
                        param_type = "string",
                        required = False,
                        description = "Namespace to query (defaults to 'default' or K8S_NAMESPACE env var)",
                    ),
                    ToolParameter(
                        name = "all_namespaces",
                        param_type = "boolean",
                        required = False,
                        default = "false",
                        description = "List pods across all namespaces",
                    ),
                ],
                handler = list_pods,
            ),
            Tool(
                name = "k8s_get_pod",
                description = "Get detailed information about a specific pod",
                parameters = [
                    ToolParameter(
                        name = "pod",
                        param_type = "string",
                        required = True,
                        description = "Name of the pod",
                    ),
                    ToolParameter(
                        name = "namespace",
                        param_type = "string",
                        required = False,
                        description = "Namespace of the pod (defaults to 'default' or K8S_NAMESPACE env var)",
                    ),
                ],
                handler = get_pod_details,
            ),
            Tool(
                name = "k8s_get_logs",
                description = "Get logs from a pod",
                parameters = [
                    ToolParameter(
                        name = "pod",
                        param_type = "string",
                        required = True,
                        description = "Name of the pod",
                    ),
                    ToolParameter(
                        name = "namespace",
                        param_type = "string",
                        required = False,
                        description = "Namespace of the pod",
                    ),
                    ToolParameter(
                        name = "container",
                        param_type = "string",
                        required = False,
                        description = "Container name (for multi-container pods)",
                    ),
                    ToolParameter(
                        name = "tail",
                        param_type = "string",
                        required = False,
                        default = "100",
                        description = "Number of lines to show from the end of the logs",
                    ),
                    ToolParameter(
                        name = "previous",
                        param_type = "boolean",
                        required = False,
                        default = "false",
                        description = "Get logs from previous container instance",
                    ),
                ],
                handler = get_pod_logs,
            ),
            Tool(
                name = "k8s_list_services",
                description = "List services in a namespace or across all namespaces",
                parameters = [
                    ToolParameter(
                        name = "namespace",
                        param_type = "string",
                        required = False,
                        description = "Namespace to query",
                    ),
                    ToolParameter(
                        name = "all_namespaces",
                        param_type = "boolean",
                        required = False,
                        default = "false",
                        description = "List services across all namespaces",
                    ),
                ],
                handler = list_services,
            ),
            Tool(
                name = "k8s_list_deployments",
                description = "List deployments in a namespace or across all namespaces",
                parameters = [
                    ToolParameter(
                        name = "namespace",
                        param_type = "string",
                        required = False,
                        description = "Namespace to query",
                    ),
                    ToolParameter(
                        name = "all_namespaces",
                        param_type = "boolean",
                        required = False,
                        default = "false",
                        description = "List deployments across all namespaces",
                    ),
                ],
                handler = list_deployments,
            ),
            Tool(
                name = "k8s_list_nodes",
                description = "List all nodes in the cluster",
                parameters = [],
                handler = list_nodes,
            ),
            Tool(
                name = "k8s_get_resource",
                description = "Get any Kubernetes resource by type and optional name",
                parameters = [
                    ToolParameter(
                        name = "type",
                        param_type = "string",
                        required = True,
                        description = "Resource type (e.g., pods, services, configmaps, secrets)",
                    ),
                    ToolParameter(
                        name = "name",
                        param_type = "string",
                        required = False,
                        description = "Resource name (omit to list all)",
                    ),
                    ToolParameter(
                        name = "namespace",
                        param_type = "string",
                        required = False,
                        description = "Namespace (for namespaced resources)",
                    ),
                    ToolParameter(
                        name = "format",
                        param_type = "string",
                        required = False,
                        default = "yaml",
                        description = "Output format (yaml, json, wide)",
                    ),
                ],
                handler = get_resource,
            ),
            Tool(
                name = "k8s_kubectl",
                description = "Execute safe kubectl commands (read-only operations)",
                parameters = [
                    ToolParameter(
                        name = "args",
                        param_type = "string",
                        required = True,
                        description = "kubectl arguments (e.g., 'get pods -A', 'describe node mynode')",
                    ),
                ],
                handler = execute_kubectl,
            ),
        ],
    )
