# Rust File Router

`FRouter` is a robust and efficient file routing tool developed in Rust, designed to automate the organization, hashing, and logging of files across different directories based on user-defined rules. Leveraging Rust's performance, safety, and concurrency, FRouter offers a high-performance solution to manage file systems more effectively.

## Features
- **Customizable File Routing**: Users can set up rules in a configuration file to automate the routing of files to specific directories based on their extensions, ensuring organized and efficient file storage.
- **SHA256 Hashing for Integrity**: Implements SHA256 hashing to ensure file integrity by checking for duplicates, guaranteeing that only unique files are routed to their specified locations.
- **Comprehensive Logging**: Detailed logs for both file events and errors are maintained, supporting JSON and database logging to facilitate easy monitoring and auditing of the routing process.
- **Case-Insensitive File Extension Handling**: Treats file extensions in a case-insensitive manner, ensuring consistent routing regardless of extension case variations.
- **Dynamic Configuration**: Dynamically loads configuration files, allowing for flexible specification of routing rules and directory paths. FRouter also supports the generation of a default configuration if none is present.
- **Efficient File Processing**: Benefits from Rust's powerful IO and concurrency capabilities, making file routing both fast and safe.

Getting Started
To start using FRouter, first ensure you have the Rust toolchain installed on your machine. Then, follow these steps:

```bash
git clone https://github.com/unixtime/frouter.git
cd frouter
cargo build --release
cp target/release/frouter /usr/local/bin
```

Set up your file routing rules in the config.toml file within the project root. Once configured, run FRouter using:

```bash
cargo run
```

### Configuration

```toml
[directories]
downloads = "~/Downloads"

[[extensions]]
name = "pdf"
path = "/Volumes/ExternalDrive/Documents/PDF"
enabled = true

[[extensions]]
name = "jpg"
path = "/Volumes/ExternalDrive/Images/JPG"
enabled = true
```

This configuration specifies that `PDF` and `JPG` files should be routed to separate directories under `~/Downloads`.

#### Create a startup file

`~/Library/LaunchAgents/com.DOMAIN.frouter.plist`

```xml
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>Label</key>
    <string>com.DOMAIN.frouter</string>
    <key>ProgramArguments</key>
    <array>
        <string>/usr/local/bin/frouter</string>
    </array>
    <key>RunAtLoad</key>
    <true/>
    <key>KeepAlive</key>
    <true/>
    <key>StandardOutPath</key>
    <string>/usr/local/var/logs/logfile.log</string>
    <key>StandardErrorPath</key>
    <string>/usr/local/var/logs/error.log</string>
</dict>
</plist>
```

#### Create an alias to run and stop the `File Router`

```bash
alias run_frouter='launchctl load ~/Library/LaunchAgents/com.DOMAIN.frouter.plist'
alias stop_frouter='launchctl unload ~/Library/LaunchAgents/com.DOMAIN.frouter.plist'
```

> Note: Replace DOMAIN with whatever you like:
> Example: com.mydomain.frouter.plist