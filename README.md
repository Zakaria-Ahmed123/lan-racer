# 🌐 NetWeave

**NetWeave** is a peer-to-peer networking system that combines
**WebRTC**, a **TUN-based virtual network interface**, and a **signaling
server** to create a lightweight, decentralized LAN/VPN-like system.

It enables devices to: - Connect directly over the internet (P2P) -
Exchange packets through a virtual network interface (TUN) - Send chat
messages alongside network traffic - Route data dynamically between
peers

------------------------------------------------------------------------

# 🚀 Features

-   🔗 Peer-to-peer WebRTC connections (DataChannel)
-   🌐 NAT traversal using STUN (and optional TURN)
-   🧭 Custom packet routing via TUN interface
-   💬 Built-in chat over peer connections
-   📡 Signaling server for SDP exchange
-   🖥️ Optional Iced GUI for control & monitoring

# ⚙️ Components

## 1. PeerManager

Handles: - WebRTC peer connections - DataChannels - Offer/Answer
negotiation - Message routing between peers

## 2. Router

Handles: - TUN device I/O - Packet routing between OS and peers -
Command execution (connect, chat, etc.) - Event processing

## 3. Signaling Server

Handles: 
- Exchange of SDP offers/answers
- Peer discovery coordination

## 4. (Optional) Iced UI

Provides:
- Peer management dashboard 
- Chat interface
- Logs and connection status

------------------------------------------------------------------------

# 📦 Example Commands

### Run the sinaling server 
``` bash
sudo cargo run --bin signal_server
```
### Create a connection

``` bash
connect peer-1
```

### Send chat

``` bash
chat peer-1 hello
```

------------------------------------------------------------------------

# 🧪 Testing Setup

## Same machine

### Run the sinaling server 
``` bash
sudo cargo run --bin signal_server
```
### Register peers to the server (use different terminals for the server , peer-1,peer-2 ,and so on) 
``` bash
cargo run --bin router -- tun0 10.10.0.1
cargo run --bin router -- tun1 10.10.0.2
```
### Connection between peers (peer-1 -> peer-2) 
- On peer-1 terminal run the following command
``` bash
        connect peer-2 
```
- You will see the following output on peer-1 and peer-2 terminals :
``` bash
        [System]: Peer 1 connected.
        [System]: Peer 2 connected. 
```
- Then you can test :
   ``` bash
        chat peer-1 <your_message>
        chat peer-2 <your_message> 
```
- Then You will see the following output on peer-1 and peer-2 terminals :
``` bash
        [Chat]: peer-1 message
        [Chat]: peer-2 message 
```

## Different machines (LAN or internet)

-   Run signaling server on public IP or VPS
-   Connect both peers to: `<server-ip>`{=html}:9000

------------------------------------------------------------------------

# 🌍 Networking Model

NetWeave uses a hybrid model:

-   **Signaling server (centralized)** → only for connection setup
-   **WebRTC (decentralized)** → actual data transfer
-   **TUN interface (local)** → integrates with OS networking stack

------------------------------------------------------------------------

# 🔐 Security

-   WebRTC encrypted transport (DTLS)
-   No direct packet exposure on signaling server
-   Peer-to-peer data flow after handshake

------------------------------------------------------------------------

# 📚 Tech Stack

-   Rust 🦀
-   Tokio (async runtime)
-   WebRTC (data channels)
-   tun-rs (virtual network interface)
-   Iced (UI framework)

------------------------------------------------------------------------

# 🧭 Roadmap

- [ ]   Add TURN support for strict NAT environments
- [ ] Improve peer discovery system
- [ ]  Add authentication layer
- [ ] Build full chat UI
- [ ]  Add network visualization (graph view)
- [ ] Add multi-peer routing (mesh mode)

------------------------------------------------------------------------

# 💡 Vision

NetWeave aims to become a lightweight, programmable mesh network layer.

------------------------------------------------------------------------

Early development --- core networking and routing in progress
