# BoaAI SSH Puzzle

This project is now a single interactive terminal puzzle meant for SSH access.

Flow:
1. User SSHs into the puzzle host/port.
2. Splash screen shows the BoaAI ASCII logo with `HACK THE WORLD` for a few seconds.
3. User solves a simplified Utility-Closet-inspired indicator puzzle.
4. On success, user enters email and confirms invite submission.

The puzzle is a custom 6-indicator variant:
- All indicators start at `OFF`
- Each SSH session gets a random target generated from 6 simulated button presses
- Explicit in-app rules panel
- Built-in `Hint` button

## Controls

Puzzle phase (no typed input):
- `Left/Right`: move focus across buttons
- `Up/Down`: switch between indicator row and action row
- `Enter` or `Space`: press the selected button
- `Esc`: quit session

Email phase:
- Type email into the input field
- `Tab`: switch focus between input and buttons
- `Enter` or `Space`: activate selected button (`Confirm Invite` or `Solve Again`)

## Run Locally

```bash
cargo run
```

Optional environment variables:
- `BOAAI_DEBUG=1`: enables debug hotkey `F12` for instant solve.
- `BOAAI_INVITE_FILE=/path/to/invite_submissions.csv`: custom submission output file.

## Quick Solve For Debugging

```bash
BOAAI_DEBUG=1 cargo run
```

Inside the puzzle UI, press:
- `F12` to auto-complete the puzzle immediately
- Then type email and activate `Confirm Invite`

## Offline Target Solver

Use `solution.py` to compute the shortest sequence from all `OFF` to any target:

```bash
python solution.py --target "WHITE,PURPLE,GREEN,WHITE,PURPLE,GREEN"
```

or

```bash
python solution.py --target "5,4,1,5,4,1"
```

## Deploy On Remote Server Without Touching Main SSH Port 22

Use a second `sshd` instance on a different port (example: `2222`) and force the puzzle command for one dedicated user.

1. Build and install binary:
```bash
cd /opt/boaai/BoaAI-Puzzle
cargo build --release
sudo install -m 755 target/release/ssh_store /opt/boaai/boaai-puzzle
```

2. Create dedicated SSH user:
```bash
sudo useradd -m -s /usr/sbin/nologin puzzle
sudo mkdir -p /home/puzzle/.ssh
sudo chown -R puzzle:puzzle /home/puzzle/.ssh
sudo chmod 700 /home/puzzle/.ssh
```
Add keys to `/home/puzzle/.ssh/authorized_keys` and set `chmod 600`.

3. Create a separate SSH config at `/etc/ssh/sshd_config_puzzle`:
```conf
Port 2222
ListenAddress 0.0.0.0
Protocol 2
HostKey /etc/ssh/ssh_host_ed25519_key
PidFile /run/sshd-puzzle.pid
PasswordAuthentication no
KbdInteractiveAuthentication no
PermitRootLogin no
AllowUsers puzzle
AuthorizedKeysFile .ssh/authorized_keys
UsePAM yes

Match User puzzle
    ForceCommand /opt/boaai/boaai-puzzle
    PermitTTY yes
    AllowTcpForwarding no
    X11Forwarding no
```

4. Validate and start separate daemon:
```bash
sudo /usr/sbin/sshd -f /etc/ssh/sshd_config_puzzle -t
sudo /usr/sbin/sshd -f /etc/ssh/sshd_config_puzzle
```

5. Open firewall for puzzle port only:
```bash
sudo ufw allow 2222/tcp
```

6. Point domain/subdomain DNS (for example `puzzle.example.com`) to the server IP and connect:
```bash
ssh -p 2222 puzzle@puzzle.example.com
```

Port `22` and your normal SSH workflow stay unchanged.
