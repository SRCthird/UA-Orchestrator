![repo-card](/resources/repo_card.png)

# UA-Orchestrator
A command-line tool for automating OPC UA node interactions using a CSV action script. Built with Rust, it connects to an OPC UA server and executes a sequence of read, write, and wait operations defined by the selected CSV file.

## Overview

This tool connects to an OPC UA server using credentials and security settings defined in a TOML configuration file. Once connected, it reads a CSV file of actions and executes them in order — reading node values, writing values, waiting for user input, or polling until a node reaches a target value.

## Getting Started

### Prerequisites

- Rust toolchain (`cargo`)
- Access to an OPC UA server

### Build

```bash
cargo build --release
```

### Configure

Copy the example configuration file and fill in your server details:

```bash
cp Config.example.toml Config.toml
```

Edit `Config.toml`:

```toml
server_url = "opc.tcp://your-server:4840"
server_security_policy = "Basic256Sha256"
server_security_mode = "SignAndEncrypt"
username = "your_username"
password = "your_password"
```

**Supported security policies:** `None`, `Basic128Rsa15`, `Basic256`, `Basic256Sha256`

**Supported security modes:** `None`, `Sign`, `SignAndEncrypt`

### Prepare your CSV action file

Copy the example CSV and customise it:

```bash
cp actions.example.csv actions.csv
```

## Usage

Run the binary and optionally pass the path to your CSV file as an argument:

```bash
./ua-orchestrator actions.csv
```

If no path is provided, the tool will prompt you to enter one interactively.

![usage-1](/resources/Screenshot-01.png)

## CSV Action Format

The CSV file must have the following columns:

```
action, tag, value, sleep
```

| Column | Description |
|---|---|
| `action` | The operation to perform (see table below) |
| `tag` | The OPC UA node identifier string (namespace index 2) |
| `value` | Optional value used by write/wait actions |
| `sleep` | Milliseconds to wait **after** the action completes |

### Actions

| Action | Description |
|---|---|
| `read` | Reads and prints the current value of the node |
| `write` | Writes `value` to the node |
| `user_write` | Prompts the user for input (uses `value` as a pre-fill if provided), then writes to the node |
| `comment` | Prints `tag` as a label/message in the terminal |
| `wait` | Pauses execution and waits for the user to press Enter |
| `wait_until` | Polls the node until its value equals `value`, then continues |
| `# ...` | Lines whose action begins with `#` are treated as in-script comments and skipped |

### Value Type Inference

Values in the CSV are automatically parsed into the appropriate OPC UA `Variant` type:

| Input | Type |
|---|---|
| `$*` | `String` where $ is dropped |
| `true` / `false` (case-insensitive) | `Boolean` |
| Integer string (e.g. `42`) | `Int64` |
| Floating-point string (e.g. `3.14`) | `Double` |
| Anything else | `String` |

### Example CSV

```csv
action,     tag,               value, sleep
comment,    Starting sequence, ,      0
write,      MyNode.SetPoint,   100,   500
read,       MyNode.SetPoint,   ,      0
write,      MyNode.SSN,        $0100, 500
read,       MyNode.SetPoint,   ,      0
wait_until, MyNode.Status,     1,     250
comment,    Done,              ,      0
```

## PKI / Certificates

On first run, a self-signed certificate and private key are automatically generated for the current OS user. They are stored at:

```
./pki/own/     ← user certificate
./pki/private/ ← private key
```

The server's certificate is automatically trusted (`trust_server_certs = true`). If your server requires manual trust configuration, add the generated certificate to the server's trusted store.

## Project Structure

| File | Description |
|---|---|
| `main.rs` | Entry point: loads config, parses args, starts OPC UA session, runs CSV |
| `config.rs` | Deserialises `Config.toml`; maps policy/mode strings to OPC UA types |
| `actions.rs` | CSV row definition, value parsing, per-row action dispatch, `run_csv` loop |
| `opc_ua_client.rs` | `OpcUaClient` trait, `OpcUaSession` with read/write batching, `LiveSessionBackend` |
| `globals.rs` | Centralised string messages, magic values, and path constants |
| `reader.rs` | `InputReader` trait and `StdinReader` implementation |
| `Config.example.toml` | Template configuration file - **copy to `Config.toml` before use** |
| `actions.example.csv` | Template action script - **copy to `actions.csv` before use** |

## Running Tests

Unit tests cover value parsing, action dispatch, and OPC UA session read/write behaviour using in-memory fakes (no live server required):

```bash
cargo test
```

## Deployment Note
If the compiled binary is moved to a system or environment without [OpenSSL 4](https://github.com/openssl/openssl/releases/tag/openssl-4.0.0) installed, the following OpenSSL DLLs must be present either in the same directory as the executable or somewhere on the system PATH:

- libcrypto-4-x64.dll
- libssl-4-x64.dll

![Deployment-Note](/resources/Screenshot-02.png)

## Copyright

Copyright 2026 Merck KGaA, Darmstadt, Germany and/or its affiliates. All rights reserved.
