<img src="https://github.com/Kerimniy/ServerStatMonitoring/blob/main/app_win_x86_64/static/favicon.png" style="width: 70px">

# System Monitor — Lightweight Real-Time System Monitoring Web App in Rust

A simple single-page real-time dashboard for monitoring your computer (CPU, RAM, disks, OS).  
Works on **Windows** and **Linux**.

<p align="center">
<img src="https://github.com/Kerimniy/ServerStatMonitoring/blob/main/prev.png" alt="logo" width="90%">
</p>

## Features

- Web interface built with **Rocket.rs** + **Handlebars** templates
- Data refreshes every **10 seconds** without page reload
- Charts showing CPU and RAM usage for the last minute
- Windows & Linux support (automatic OS name detection)
- No external server required — runs as a single binary
- Minimal dependencies, very small footprint

## Technologies Used

- **Rocket** — web framework
- **sysinfo** — system information collection
- **tokio** — background asynchronous data updates
- **serde + rocket_dyn_templates** — JSON API and templating
- **chrono** — time handling
