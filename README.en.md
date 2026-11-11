# execenv

`execenv` is a small Unix‑oriented entrypoint utility for containers.
It substitutes environment variable values into `${VAR}` placeholders in one or more files, and then can replace its own process with another process via `exec(2)`.
This is useful when an application needs a static configuration file, but the final values are only known at container startup time.

## Motivation

`execenv` is primarily designed for container images where configuration must be provided at runtime rather than at build time.
A typical example is a frontend or an application based on a distroless image. The same image can be used across multiple environments: development, staging, and production. Instead of rebuilding the image for each environment, you can store a configuration template inside the image and render the final file when the container starts.
The utility solves the problem of minimalistic containers that lack a shell and standard Unix utilities. In such containers, you cannot use the usual pipeline like `envsubst < config.template > config.yaml && exec nginx -g "daemon off;"`.
At the same time, `execenv` has a very small footprint—less than 1MB.

## Features

- Substitution of `${VAR}` from environment variables.
- Processing of one or several files in a single run.
- In‑place file overwriting or rendering from an input file to a separate output file.
- Configurable behavior when an environment variable is missing.
- Writing via a temporary file, replacing the target file only after successful rendering.
- Optional transition to the final application process via `exec(2)`.
- Optional removal of variables from the executed command's environment.
- Primary use case: Linux/Unix containers.

## Limitations

> [!IMPORTANT]
> - Default read buffer size is 256KiB.
> - Maximum length of a variable name in a placeholder is 4KiB. Unfinished placeholders longer than this are preserved as plain text.
> - The output file inherits access permissions from the input file.
> - Maximum number of filtered environment variables is 10 000.
> - All `execenv` options must appear before `--exec`.
> - Only Unix‑like systems are supported.

## Installation

### Build from source

```sh
cargo build --release
```

The optimized binary will be available at:

```sh
target/release/execenv
```

The project requires **Rust1.91** or later.

### Download a stable release

The compiled binary of the latest stable release is available at https://github.com/dsk13-stack/execenv/releases.

## Usage

```sh
execenv --files <file> [<file> ...] [--missing-env <mode>] [--exec <command> [args...]]
```

Short flags are also available:

```sh
execenv -f <file> [-m <mode>] [-e <command> [args...]]
```

### File arguments

A regular file path is rendered in place:

```sh
execenv --files /app/config.yaml
```

Mapping renders one file into another:

```sh
execenv --files /templates/config.yaml=/app/config.yaml
```

You can process multiple files in a single run:

```sh
execenv --files /app/config.yaml /app/worker.yaml
```

Or multiple mapping arguments:

```sh
execenv --files \
  /templates/app.yaml=/app/config.yaml \
  /templates/worker.yaml=/app/worker.yaml
```

## Missing variables

The `--missing-env` flag controls what happens if a file contains a reference to an environment variable that is not set.

Available modes:

| Mode | Behavior |
| --- | --- |
| `empty` | Replace the placeholder with an empty string. This is the default. |
| `keep` | Leave the original placeholder `${VAR}` unchanged. |
| `error` | Exit immediately with an error and do not run the command. |

Example:

```sh
execenv --files /app/config.yaml --missing-env error
```

For production environments, it is usually safer to use `--missing-env error` so that the application does not start with incomplete configuration.

## Running a command

If `--exec` is provided, `execenv` first renders all specified files. If rendering succeeds, the utility replaces its own process with the specified command via `exec(2)`.

```sh
execenv \
  --files /templates/nginx.conf=/etc/nginx/nginx.conf \
  --missing-env error \
  --exec nginx -g "daemon off;"
```

Everything after `--exec` is treated as the command and its arguments, including values that start with `-`.

If rendering fails, the command is not launched.

## Example

Template file:

```yaml
server:
  host: ${APP_HOST}
  port: ${APP_PORT}
```

Run:

```sh
APP_HOST=0.0.0.0 APP_PORT=8080 \
execenv --files /templates/config.yaml=/app/config.yaml
```

Resulting file:

```yaml
server:
  host: 0.0.0.0
  port: 8080
```

## The executed command's environment

By default, the command inherits the entire environment of `execenv`. If variables are only needed to render configuration, you can remove them from the environment of the command executed via `--exec`:

- `--unset-rendered-envs` (`-u`): remove variables referenced by placeholders across all processed templates.
- `--excluded-envs VAR1,VAR2` (`-x VAR1,VAR2`): remove explicitly named variables, even if they are not referenced in the templates.
- Both flags together: remove the union of these sets of variables.

Variables remain available during rendering. Removal only applies to the executed command's environment.

For example, remove variables referenced in the template and also exclude `BOOTSTRAP_TOKEN`:

```sh
execenv \
  --files /templates/app.yaml=/app/config.yaml \
  --unset-rendered-envs \
  --excluded-envs BOOTSTRAP_TOKEN \
  --exec /app/server
```

Use `--unset-rendered-envs` when the executed command no longer needs the variables referenced in the templates. 
If a template contains `${HOME}` or `${PATH}`, those variables are also removed from the command's environment. 
Use `--excluded-envs` to choose exactly which variables to remove.

The total number of unique variables to remove from `--excluded-envs` and all templates must not exceed 10,000. 
Repeated names count only once. 
Exceeding the limit aborts processing and prevents the command from running, 
regardless of placeholder defaults or the `--missing-env` mode


## Example Dockerfile

```dockerfile
FROM chainguard/nginx:latest

COPY target/release/execenv /usr/bin/execenv
COPY templates/nginx.conf /templates/nginx.conf

ENTRYPOINT ["/usr/bin/execenv"]
CMD ["--files", "/templates/nginx.conf=/etc/nginx/nginx.conf", "--missing-env", "error", "--exec", "/usr/sbin/nginx", "-c", "/etc/nginx/nginx.conf", "-e", "/dev/stderr", "-g", "daemon off;"]
```

## Placeholder syntax

`execenv` uses Spring Boot‑style placeholder syntax.

Two forms are supported:

```text
${VAR}
${VAR:default}
```

Examples:

```text
${DATABASE_URL}
${APP_PORT:8080}
${LOG_LEVEL:info}
${API_URL:https://localhost:8080/api}
```

Behavior:

| Placeholder | Variable | Result |
|---|---|---|
| `${VAR}` | unset | depends on `--missing-env` |
| `${VAR}` | `""` | empty string |
| `${VAR:}` | unset | empty string |
| `${VAR:default}` | unset | `default` |
| `${VAR:default}` | `""` | empty string |
| `${VAR:default}` | set | variable value |

If a default value is specified after `:`, the `--missing-env` flag does not apply to that placeholder.

Not supported:

```text
$VAR
${VAR:-default}
${VAR-default}
${OUTER.${INNER}}
```

## Security notes

`execenv` does not evaluate shell expressions and does not execute environment variable values as code. The command after `--exec` is launched directly as a program with arguments, not via `sh -c`.

Removing variables from the command's environment reduces further propagation of secrets through the environment of the application and any programs it launches. Values substituted into configuration remain in the generated file.
Be careful when writing final configurations to shared volumes, persistent storage, logs, support bundles, or directories accessible to other processes.

## Supported platforms

`execenv` is intended for Unix‑like systems because `--exec` uses `exec(2)`. The primary target environment is Linux containers.


## License

Licensed under the MIT License.
