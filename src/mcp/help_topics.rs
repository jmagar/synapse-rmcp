//! Static MCP help topic documentation.

// docker: subactions

pub(super) const DOCKER_INFO: &str = "\
Query Docker daemon info on one or all configured hosts.

**Parameters**
- `host` (optional): target host name. Omit to fan out across all configured hosts.

**Returns** daemon version, OS, kernel, memory, CPU count, and plugin info.";

pub(super) const DOCKER_DF: &str = "\
Show Docker disk usage (images, containers, volumes, build cache) on one or all hosts.

**Parameters**
- `host` (optional): target host name.";

pub(super) const DOCKER_IMAGES: &str = "\
List Docker images on one or all configured hosts.

**Parameters**
- `host` (optional): target host name.
- `dangling_only` (bool, optional): only list untagged (dangling) images.";

pub(super) const DOCKER_NETWORKS: &str = "\
List Docker networks on one or all configured hosts.

**Parameters**
- `host` (optional): target host name.";

pub(super) const DOCKER_VOLUMES: &str = "\
List Docker volumes on one or all configured hosts.

**Parameters**
- `host` (optional): target host name.";

pub(super) const DOCKER_PULL: &str = "\
Pull a Docker image on a specific host. **Requires confirmation.**

**Parameters**
- `host` (required): target host name.
- `image` (required): image reference, e.g. `nginx:latest`.";

pub(super) const DOCKER_BUILD: &str = "\
Build a Docker image on a specific host. **Destructive — requires confirmation.**

**Parameters**
- `host` (required): target host name.
- `context` (required): absolute build context path (no `..`, `~`, or `$` expansion).
- `tag` (required): image tag (`-t`).
- `dockerfile` (optional): Dockerfile path relative to context.
- `no_cache` (bool, optional): pass `--no-cache`.";

pub(super) const DOCKER_RMI: &str = "\
Remove a Docker image on a specific host. **Destructive — requires confirmation.**

**Parameters**
- `host` (required): target host name.
- `image` (required): image reference.
- `force` (required, must be `true`): safety guard.";

pub(super) const DOCKER_PRUNE: &str = "\
Prune Docker resources on a specific host. **Destructive — requires confirmation.**

**Parameters**
- `host` (required): target host name.
- `prune_target` (required): one of `containers`, `images`, `volumes`, `networks`, `buildcache`, `all`.
- `force` (required, must be `true`): safety guard.";

// container: subactions

pub(super) const CONTAINER_LIST: &str = "\
List containers on one or all configured hosts.

**Parameters**
- `host` (optional): target host name.
- `state` (optional): filter by `running` | `exited` | `paused` | `restarting` | `all` (default `all`).
- `name_filter` (optional): partial match on container name.
- `image_filter` (optional): partial match on image.
- `label_filter` (optional): label match in `key=value` form.";

pub(super) const CONTAINER_INSPECT: &str = "\
Inspect a container on a host.

**Parameters**
- `host` (required): target host name.
- `container_id` (required): container id or name.
- `summary` (bool, optional): abbreviated info only.";

pub(super) const CONTAINER_LOGS: &str = "\
Retrieve container logs.

**Parameters**
- `host` (required): target host name.
- `container_id` (required): container id or name.
- `lines` (int, optional): tail line count (default 50).
- `since` / `until` (optional): ISO 8601 / unix seconds / duration (e.g. `30m`).
- `grep` (optional): keep only lines containing this substring.
- `stream` (optional): `stdout` | `stderr` | `both` (default `both`).";

pub(super) const CONTAINER_STATS: &str = "\
Show live CPU / memory / network / IO stats for containers on a host.

**Parameters**
- `host` (optional): target host name.
- `container_id` (optional): restrict to a single container.";

pub(super) const CONTAINER_TOP: &str = "\
Show running processes inside a container (`docker top`).

**Parameters**
- `host` (required): target host name.
- `container_id` (required): container id or name.";

pub(super) const CONTAINER_SEARCH: &str = "\
Full-text search over container names, images, and labels.

**Parameters**
- `host` (optional): target host name.
- `query` (required): search query.";

pub(super) const CONTAINER_START: &str = "\
Start a stopped container.

**Parameters**
- `host` (required): target host name.
- `container_id` (required): container id or name.";

pub(super) const CONTAINER_STOP: &str = "\
Stop a running container. **Destructive — requires confirmation.**

**Parameters**
- `host` (required): target host name.
- `container_id` (required): container id or name.";

pub(super) const CONTAINER_RESTART: &str = "\
Restart a container.

**Parameters**
- `host` (required): target host name.
- `container_id` (required): container id or name.";

pub(super) const CONTAINER_PAUSE: &str = "\
Pause a container (freeze all processes).

**Parameters**
- `host` (required): target host name.
- `container_id` (required): container id or name.";

pub(super) const CONTAINER_RESUME: &str = "\
Resume a paused container.

**Parameters**
- `host` (required): target host name.
- `container_id` (required): container id or name.";

pub(super) const CONTAINER_PULL_IMG: &str = "\
Pull the latest image for a container without recreating it.

**Parameters**
- `host` (required): target host name.
- `container_id` (required): container id or name.";

pub(super) const CONTAINER_RECREATE: &str = "\
Recreate a container (stop → pull → start). **Destructive — requires confirmation.**

**Parameters**
- `host` (required): target host name.
- `container_id` (required): container id or name.
- `pull` (bool, optional): pull latest image before recreating (default `true`).";

pub(super) const CONTAINER_EXEC: &str = "\
Execute a command inside a running container. **Destructive — requires confirmation.**

**Parameters**
- `host` (required): target host name.
- `container_id` (required): container id or name.
- `command` (array of strings, required): argv — index 0 is the binary, rest are args. No shell.
- `exec_user` (optional): user to run as inside the container.
- `exec_workdir` (optional): working directory inside the container.
- `exec_timeout_ms` (int, optional): timeout in milliseconds [1000, 300000] (default 30000).";

// host: subactions

pub(super) const HOST_STATUS: &str = "\
Quick health check for one or all hosts.

**Parameters**
- `host` (optional): target host name.";

pub(super) const HOST_INFO: &str = "\
Detailed host information (OS, kernel, hardware).

**Parameters**
- `host` (optional): target host name.";

pub(super) const HOST_UPTIME: &str = "\
Host uptime and load averages.

**Parameters**
- `host` (optional): target host name.";

pub(super) const HOST_RESOURCES: &str = "\
CPU and memory usage summary.

**Parameters**
- `host` (optional): target host name.";

pub(super) const HOST_SERVICES: &str = "\
List systemd services.

**Parameters**
- `host` (required): target host name.
- `state` (optional): filter by service state.
- `service` (optional): filter by service name substring.";

pub(super) const HOST_NETWORK: &str = "\
Network interfaces and addresses.

**Parameters**
- `host` (optional): target host name.";

pub(super) const HOST_MOUNTS: &str = "\
Mounted filesystems.

**Parameters**
- `host` (required): target host name.";

pub(super) const HOST_PORTS: &str = "\
Listening TCP/UDP ports.

**Parameters**
- `host` (required): target host name.
- `protocol` (optional): `tcp` | `udp`.
- `limit` / `offset` (int, optional): pagination.";

pub(super) const HOST_DOCTOR: &str = "\
Pre-flight connectivity checks for a host.

**Parameters**
- `host` (required): target host name.
- `checks` (optional): comma-separated check names to run.";

// compose: subactions

pub(super) const COMPOSE_LIST: &str = "\
List discovered compose projects on a single host.

**Parameters**
- `host` (required): target host name.";

pub(super) const COMPOSE_STATUS: &str = "\
Show status of a compose project.

**Parameters**
- `host` (required): target host name.
- `project` (required): compose project name.
- `service` (optional): restrict to a single service.";

pub(super) const COMPOSE_UP: &str = "\
Start a compose project.

**Parameters**
- `host` (required): target host name.
- `project` (required): compose project name.";

pub(super) const COMPOSE_DOWN: &str = "\
Stop and remove a compose project. **Destructive — requires confirmation.**

**Parameters**
- `host` (required): target host name.
- `project` (required): compose project name.
- `remove_volumes` (bool, optional): also remove named volumes. Requires `force=true`.
- `force` (bool): required when `remove_volumes=true`.";

pub(super) const COMPOSE_RESTART: &str = "\
Restart a compose project. **Destructive — requires confirmation.**

**Parameters**
- `host` (required): target host name.
- `project` (required): compose project name.";

pub(super) const COMPOSE_RECREATE: &str = "\
Recreate a compose project (pull + down + up). **Destructive — requires confirmation.**

**Parameters**
- `host` (required): target host name.
- `project` (required): compose project name.";

pub(super) const COMPOSE_LOGS: &str = "\
Retrieve logs for a compose project.

**Parameters**
- `host` (required): target host name.
- `project` (required): compose project name.
- `service` (optional): restrict to a single service.
- `lines` (int, optional): tail line count.
- `since` (optional): start time (ISO 8601 / duration).";

pub(super) const COMPOSE_BUILD: &str = "\
Build images for a compose project.

**Parameters**
- `host` (required): target host name.
- `project` (required): compose project name.
- `service` (optional): restrict to a single service.";

pub(super) const COMPOSE_PULL: &str = "\
Pull images for a compose project.

**Parameters**
- `host` (required): target host name.
- `project` (required): compose project name.
- `service` (optional): restrict to a single service.";

pub(super) const COMPOSE_REFRESH: &str = "\
Invalidate the compose project discovery cache for a host.

**Parameters**
- `host` (optional): target host name. Omit to invalidate all hosts.";

// scout: simple actions

pub(super) const SCOUT_NODES: &str = "\
List all configured hosts (nodes).

No parameters required.

**Returns** host name, protocol, address, port, and tags for each host.";

pub(super) const SCOUT_PEEK: &str = "\
Peek at a file or directory on a host.

**Parameters**
- `host` (required): target host name.
- `path` (required): absolute path to peek at.
- `tree` (bool, optional): emit a depth-limited directory tree.
- `depth` (int, optional): tree depth [1, 20] (default 3).";

pub(super) const SCOUT_FIND: &str = "\
Glob search for files on a host.

**Parameters**
- `host` (required): target host name.
- `path` (required): starting directory (absolute).
- `pattern` (required): glob pattern for `-name` (must not start with `-`).
- `depth` (int, optional): max depth [1, 20] (default 10).
- `limit` (int, optional): max results (default 500).";

pub(super) const SCOUT_PS: &str = "\
List running processes on a host.

**Parameters**
- `host` (required): target host name.
- `sort` (optional): `cpu` | `mem` | `pid` | `time` (default `cpu`).
- `grep` (optional): substring filter on process lines.
- `user` (optional): prefix-match filter on user column.
- `limit` (int, optional): max results (default 50).";

pub(super) const SCOUT_DF: &str = "\
Disk usage on a host.

**Parameters**
- `host` (required): target host name.
- `path` (optional): restrict to a specific mount point.";

pub(super) const SCOUT_DELTA: &str = "\
Diff a file between two hosts (or against inline content).

**Parameters**
- `source_host` (required): source host name.
- `source_path` (required): source file path (absolute).
- `target_host` + `target_path` OR `content`: destination. Mutually exclusive.";

pub(super) const SCOUT_EXEC: &str = "\
Execute an allowlisted command on a host. **Destructive — requires confirmation.**

**Allowlist**: `cat`, `head`, `tail`, `grep`, `rg`, `find`, `ls`, `tree`, `wc`, `sort`, `uniq`, `diff`, `stat`, `file`, `du`, `df`, `pwd`, `hostname`, `uptime`, `whoami`.

**Parameters**
- `host` (required): target host name.
- `command` (required): command name from allowlist.
- `args` (array, optional): positional arguments (execvp-style, no shell).
- `path` (optional): working directory (local hosts only).
- `timeout_secs` (int, optional): per-host timeout (default 30).";

pub(super) const SCOUT_EMIT: &str = "\
Run an allowlisted command across multiple hosts. **Destructive — requires confirmation.**

**Parameters**
- `targets` (array of `{host, path?}`): target hosts.
- `command` (required): allowlisted command name.
- `args` (array, optional): positional arguments.
- `timeout_secs` (int, optional): per-host timeout (default 30).";

pub(super) const SCOUT_BEAM: &str = "\
Transfer a file between hosts. **Destructive — requires confirmation.**

**Parameters**
- `host` (required): source host name.
- `path` (required): source file path (absolute).
- `dest_host` (required): destination host name.
- `dest_path` (required): destination path (absolute).";

// zfs: subactions

pub(super) const ZFS_POOLS: &str = "\
List ZFS pools on a host.

**Parameters**
- `host` (required): target host name.
- `pool` (optional): exact pool name filter.";

pub(super) const ZFS_DATASETS: &str = "\
List ZFS datasets on a host.

**Parameters**
- `host` (required): target host name.
- `pool` (optional): restrict to this pool (`-r`).
- `dataset_type` (optional): `filesystem` | `volume` | `snapshot` | `bookmark` | `all`.
- `recursive` (bool, optional): list recursively (default false).";

pub(super) const ZFS_SNAPSHOTS: &str = "\
List ZFS snapshots on a host.

**Parameters**
- `host` (required): target host name.
- `pool` (optional): restrict to this pool.
- `dataset` (optional): restrict to this dataset (takes priority over pool).
- `limit` (int, optional): max results.";

// logs: subactions

pub(super) const LOGS_SYSLOG: &str = "\
Retrieve `/var/log/syslog` (or `/var/log/messages`) on a host.

**Parameters**
- `host` (required): target host name.
- `lines` (int, optional): line count [1, 500] (default 100).
- `grep` (optional): substring filter (injection-safe, applied locally).";

pub(super) const LOGS_JOURNAL: &str = "\
Query the systemd journal on a host.

**Parameters**
- `host` (required): target host name.
- `lines` (int, optional): line count [1, 500] (default 100).
- `unit` (optional): systemd unit filter (`-u`).
- `priority` (optional): priority filter (`-p`). E.g. `err`, `warning`, `info`.
- `since` (optional): start time (`--since`). E.g. `2026-05-29 00:00:00` or `-1h`.
- `until` (optional): end time (`--until`).
- `grep` (optional): substring filter.";

pub(super) const LOGS_DMESG: &str = "\
Retrieve kernel ring buffer (`dmesg`) on a host.

**Parameters**
- `host` (required): target host name.
- `lines` (int, optional): line count [1, 500] (default 100).
- `grep` (optional): substring filter.";

pub(super) const LOGS_AUTH: &str = "\
Retrieve auth log (`/var/log/auth.log` or `journalctl` auth facility).

**Parameters**
- `host` (required): target host name.
- `lines` (int, optional): line count [1, 500] (default 100).
- `grep` (optional): substring filter.";
