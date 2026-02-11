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

## Run As Anonymous SSH Service (Port 1337)

This project now includes `ssh_gateway.py`, which:
- listens for SSH on `1337`
- disables authentication (username/password/keys not required)
- launches one puzzle process per connection (concurrent users supported)

1. Build the puzzle binary:
```bash
cargo build --release
```

2. Install gateway dependency:
```bash
python3 -m pip install --user asyncssh
```

3. Start the SSH gateway:
```bash
python3 ssh_gateway.py --host 0.0.0.0 --port 1337 --binary target/release/ssh_store
```

4. Connect from any machine:
```bash
ssh -p 1337 anything@your-domain.com
```

Notes:
- The SSH username is ignored.
- Port `22` is not touched.
- Each connected user gets an isolated puzzle session.

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

## Production Service (Systemd)

Run gateway as a managed service while leaving your normal SSH daemon on port `22`.

1. Install app on server:
```bash
sudo mkdir -p /opt/boaai
sudo chown -R $USER:$USER /opt/boaai
cd /opt/boaai
git clone <your-repo-url> BoaAI-Puzzle
cd BoaAI-Puzzle
cargo build --release
python3 -m pip install --user asyncssh
```

2. Create service file `/etc/systemd/system/boaai-puzzle-ssh.service`:
```ini
[Unit]
Description=BoaAI Anonymous SSH Puzzle Gateway
After=network.target

[Service]
Type=simple
User=ubuntu
WorkingDirectory=/opt/boaai/BoaAI-Puzzle
ExecStart=/usr/bin/python3 /opt/boaai/BoaAI-Puzzle/ssh_gateway.py --host 0.0.0.0 --port 1337 --binary /opt/boaai/BoaAI-Puzzle/target/release/ssh_store
Restart=always
RestartSec=2
Environment=BOAAI_INVITE_FILE=/opt/boaai/BoaAI-Puzzle/invite_submissions.csv

[Install]
WantedBy=multi-user.target
```

3. Enable and start:
```bash
sudo systemctl daemon-reload
sudo systemctl enable --now boaai-puzzle-ssh.service
sudo systemctl status boaai-puzzle-ssh.service
```

4. Open firewall + point DNS:
```bash
sudo ufw allow 1337/tcp
```

Then users connect with:
```bash
ssh -p 1337 anything@puzzle.example.com
```

Your main SSH service on port `22` remains unchanged.
