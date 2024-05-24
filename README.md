# Nudge

Nudge is a small cli-tool for transferring files between computers. 
This project is heavily inspired `croc` and `qft`.

## Demo

https://github.com/darmiel/nudge-ngx/assets/71837281/9c9bbdbd-b383-45cb-ad7b-383bbb0176b3

## Usage

### Server

The server is the central component of the Nudge system. 
It manages the connections between peers and facilitates the file transfer process.

```console
$ nudge serve
Starting server on 0.0.0.0:4000
```

> **Usage**
> 
> ```
> Usage: nudge-ngx serve [OPTIONS]
> 
> Options:
>   -x, --host <HOST>  [default: 0.0.0.0]
>   -p, --port <PORT>  [default: 4000]
>   -h, --help         Print help
> ```

### Send

The send command is used to upload a file to another peer.

```console
$ nudge send file.txt
Download file with passphrase: sagem-tribal-israeli
```

> **Usage**
> 
> ```
> Usage: nudge send [OPTIONS] <FILE>
> 
> Arguments:
>   <FILE>
> 
> Options:
>   -x, --relay-host <RELAY_HOST>          [default: 127.0.0.1]
>   -y, --relay-port <RELAY_PORT>          [default: 4000]
>   -d, --delay <DELAY>                    [default: 500]
>   -b, --bitrate <BITRATE>                [default: 256]
>   -h, --help                             Print help
> ```

### Get

The get command is used to download a file from another peer.

```console
$ nudge get -o file.txt sagem-tribal-israeli
```

> **Usage**
> 
> ```
> Usage: nudge-ngx get [OPTIONS] --out-file <OUT_FILE> <PASSPHRASE>
> 
> Arguments:
>   <PASSPHRASE>
> 
> Options:
>   -o, --out-file <OUT_FILE>
>   -x, --relay-host <RELAY_HOST>          [default: 127.0.0.1]
>   -y, --relay-port <RELAY_PORT>          [default: 4000]
>   -d, --delay <DELAY>                    [default: 500]
>   -b, --bitrate <BITRATE>                [default: 256]
>   -h, --help                             Print help
> ```

## Building from Source

To build Nudge from source, follow these steps:

1. Clone the repository and navigate to the cloned directory:
   ```bash
   git clone https://github.com/darmiel/nudge-ngx
   cd nudge-ngx
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

The executable will be available in `target/release/nudge-ngx`.
