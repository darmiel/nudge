# Nudge

Nudge is a small CLI tool for sending data from one computer to another via UDP. 
The project was strongly inspired by the `croc` and `qft` projects.
The aim is to enable simple peer-to-peer data transfer, so that the data is not sent via a central server, but transferred directly peer-to-peer,

## Demo

https://github.com/darmiel/nudge/assets/71837281/bfb5c9c8-a4a2-40eb-ba84-141cea2d352c

## Usage

```
Usage: nudge [OPTIONS] <COMMAND>

Commands:
  * serve
    
  * send [OPTIONS] <FILE>
    -d, --delay <DELAY>            [default: 500]
    -c, --chunk-size <CHUNK_SIZE>  [default: 4096]
        --hide-hostname            Send file as <anonymous>
        --skip-hash                Don't create a hash of the file
  
  * get [OPTIONS] <PASSPHRASE>
    -o, --out-file <OUT_FILE>      Override the output file (optional)
    -d, --delay <DELAY>            [default: 500]
    -f, --force                    Don't ask for confirmation when downloading the file
        --hide-hostname            Receive file as <anonymous>
        --overwrite-file           Overwrite the output file without asking
        --no-prompt                Don't display any prompts and quit (could be useful for scripting)
        --skip-hash                Don't perform hash check of the downloaded file
    -c, --chunk-size <CHUNK_SIZE>  Chunk size to read from the socket [default: 4096]
    
  * help

Global Options:
    -x, --relay-host <RELAY_HOST>  [env: NUDGE_RELAY_HOST=] [default: relay-1.nudge.d2a.io]
    -y, --relay-port <RELAY_PORT>  [env: NUDGE_RELAY_PORT=] [default: 80]
    -v, --verbose
    -h, --help                     Print help
    -V, --version                  Print version
```

### Server

The server acts as a relay server. 
This server should be publicly accessible (i.e. by every peer). 
The relay server manages the communication and connects the peers with each other.

You can use the following public server: `new.d2a.io:4000` (no guarantees for availability).

## Installation

### Brew

You can install nudge using Homebrew:

```bash
brew install darmiel/nudge/nudge
```

### Building from Source

To build Nudge from source, follow these steps:

1. Clone the repository and navigate to the cloned directory:
   ```bash
   git clone https://github.com/darmiel/nudge
   cd nudge
   ```

2. Compile the project with Cargo:
   ```bash
   cargo build --release
   ```

> [!NOTE]
> You may need to install a linker such as `gcc` or `clang` to compile the project.
> ```console
> sudo apt install build-essential
> ```

The executable will be available in `target/release/nudge`.

<!--
## TODO

- [x] Send file meta over socket (hostname, ...)
- [x] Send Hostname + Let the user hide the hostname
- [x] Add env vars for relay-host / -port\
- [x] Hash Check
- [x] Logger (Verbose Mode)
- [ ] AES
- [ ] Compression
- [x] Make options global
- [x] Option to overwrite file
- [x] Server should send errors
- [x] Filename by sender
- [ ] Chunk size sent by sender
-->