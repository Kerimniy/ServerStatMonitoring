//#![windows_subsystem = "windows"]
use std::path::{PathBuf, Path};
use rocket::http::Status;
use rocket::fs::{NamedFile, relative};
#[cfg(target_os = "windows")]
use std::os::windows::process::CommandExt;
use std::process::{Command, Output};
use chrono;
use std::sync::{MutexGuard, OnceLock};
use tokio::sync::Mutex;
use std::collections::{HashMap, HashSet};
use std::ops::Deref;
use chrono::{DateTime, NaiveDateTime, Offset, Utc};
use rocket::http::hyper::Version;
use rocket_dyn_templates::{Template, context};
use rocket::serde::{Serialize, json::Json, Deserialize};
use rocket::{execute, tokio, uri, Response};
use  rocket::{get, routes};
use sysinfo::{
    Components, Disks, Networks, System, RefreshKind, CpuRefreshKind
};
use rocket::response::content::RawHtml;

const  GBCONST:i32 = 1024*1024*1024;
static TZ: OnceLock<i32> = OnceLock::new();

#[derive(Clone, Serialize)]
struct Pair<T,T1>{
    v1:T,
    v2:T1
}

#[derive(Serialize, Clone)]
struct CPU_INFO{
    name:String,
    frequency:u64,
    cpu_count:u16,
    cpu_usage:f32,
    prev_cpus:Vec<Pair<i64,f32>>,

}

#[derive(Serialize, Clone)]
struct RAM_INFO{
    ram_size:u32,
    ram_used:f32,
    swap_size:u32,
    swap_used:f32,

    prev_ram_used:Vec<Pair<i64,f32>>,
}

#[derive(Serialize, Clone)]
struct DISK_INFO{
    total_size:u32,
    used:f32,
    prev_read:Vec<Pair<i64,i32>>,
    prev_write:Vec<Pair<i64,i32>>,
}

#[derive(Serialize, Clone)]
struct OS_INFO{
    name: String,
    kernel: String,
    uptime: u64,
}

static cpu_info: OnceLock<Mutex<CPU_INFO>> = OnceLock::new();

static ram_info: OnceLock<Mutex<RAM_INFO>> = OnceLock::new();

static disk_info: OnceLock<Mutex<DISK_INFO>> = OnceLock::new();

static os_info: OnceLock<Mutex<OS_INFO>> = OnceLock::new();

static sys_el: OnceLock<Mutex<System>> = OnceLock::new();

static disk_el: OnceLock<Mutex<Disks>> = OnceLock::new();

impl RAM_INFO {
    fn default() -> Self {
        Self {
            ram_size: 0,
            ram_used: 0.0,
            swap_size: 0,
            swap_used: 0.0,
            prev_ram_used: vec![],
        }
    }
}

fn init() {
    let mut sys = System::new_all();
    sys.refresh_all();

    let mut cpu_sys = System::new_with_specifics(
        RefreshKind::nothing().with_cpu(CpuRefreshKind::everything()),
    );

    let disks = Disks::new_with_refreshed_list();

    let mut total_size: u64 = 0;
    let mut available_size: u64 = 0;
    let mut total_read_mb: i64 = 0;
    let mut total_write_mb: i64 = 0;

    for disk in &disks {
        total_size = total_size.saturating_add(disk.total_space() / 1_073_741_824); // 1024^3
        available_size = available_size.saturating_add(disk.available_space() / 1_073_741_824);
        total_read_mb = total_read_mb.saturating_add((disk.usage().total_read_bytes / 1024 / 1024) as i64);
        total_write_mb = total_write_mb.saturating_add((disk.usage().total_written_bytes / 1024 / 1024) as i64);
    }

    let disk_usage_percent = if total_size > 0 {
        ((total_size - available_size) as f32 / total_size as f32) * 100.0
    } else {
        0.0
    };

    std::thread::sleep(sysinfo::MINIMUM_CPU_UPDATE_INTERVAL);

    cpu_sys.refresh_cpu_all();

    let cpus = cpu_sys.cpus();

    let cpu_count = cpus.len() as u16;
    let (cpu_freq, cpu_usage) = if !cpus.is_empty() {
        let mut total_usage = 0.0;
        let mut total_freq = 0u64;
        for cpu in cpus {
            total_usage += cpu.cpu_usage();
            total_freq += cpu.frequency();
        }
        let avg_usage = total_usage / cpus.len() as f32;
        let avg_freq = total_freq / cpus.len() as u64;
        (avg_freq, (avg_usage * 10.0).round() / 10.0)
    } else {
        (0u64, 0.0)
    };

    let cpu_brand = cpus.first()
        .map(|c| c.brand().trim().to_string())
        .unwrap_or_else(|| "Unknown CPU".to_string());

    let total_mem_gb = (sys.total_memory() / 1_073_741_824) as u32;
    let used_mem_percent = if sys.total_memory() > 0 {
        ((sys.used_memory() as f32 / sys.total_memory() as f32) * 1000.0).round() / 10.0
    } else {
        0.0
    };

    let total_swap_gb = (sys.total_swap() / 1_073_741_824) as u32;
    let used_swap_percent = if sys.total_swap() > 0 {
        ((sys.used_swap() as f32 / sys.total_swap() as f32) * 1000.0).round() / 10.0
    } else {
        0.0
    };

    let now = chrono::Local::now().timestamp();

    cpu_info.get_or_init(|| Mutex::new(CPU_INFO {
        name: cpu_brand,
        frequency: cpu_freq,
        cpu_count,
        cpu_usage,
        prev_cpus: vec![Pair { v1: now, v2: cpu_usage }],
    }));

    ram_info.get_or_init(|| Mutex::new(RAM_INFO {
        ram_size: total_mem_gb,
        ram_used: used_mem_percent,
        swap_size: total_swap_gb,
        swap_used: used_swap_percent,
        prev_ram_used: vec![Pair { v1: now, v2: used_mem_percent }],
    }));

    disk_info.get_or_init(|| Mutex::new(DISK_INFO {
        total_size: total_size.min(u32::MAX as u64) as u32,
        used: disk_usage_percent,
        prev_read: vec![Pair { v1: now, v2: total_read_mb as i32 }],
        prev_write: vec![Pair { v1: now, v2: total_write_mb as i32 }],
    }));

    #[cfg(target_os = "windows")]
    let cmd = r#"powershell -NoProfile -Command "(Get-ItemProperty 'HKLM:\SOFTWARE\Microsoft\Windows NT\CurrentVersion').ProductName""#;
    #[cfg(not(target_os = "windows"))]
    let cmd = r#"grep ^PRETTY_NAME= /etc/os-release | cut -d= -f2 | tr -d '"'"#;

    os_info.get_or_init(|| Mutex::new(OS_INFO {
        name: execute_command(cmd).trim().to_string(),
        kernel: System::kernel_version().unwrap_or("Unknown".to_string()),
        uptime: System::uptime() / 86400, // дни
    }));

    disk_el.get_or_init(|| Mutex::new(disks));
    sys_el.get_or_init(|| Mutex::new(sys));
}
#[get("/")]
async fn index()->Template {
    Template::render("index", context! { arrow_url: uri!(static_handler("/arrow.svg"))})
}

#[get("/getcpuinfo")]
async fn getcpuinfo() -> Json<CPU_INFO> {

    let info = cpu_info.get_or_init(|| Mutex::new(CPU_INFO {
        name: "Undefined CPU".to_string(),
        frequency: 0,
        cpu_count: 0,
        cpu_usage: 0.0,
        prev_cpus: Vec::new(),
    })).lock().await;
    Json(info.clone())
}

#[get("/getraminfo")]
async fn getraminfo() -> Json<RAM_INFO> {

    let info = ram_info.get_or_init(|| Mutex::new(RAM_INFO {
        ram_size: 0,
        swap_used: 0.0,
        ram_used: 0.0,
        swap_size: 0,
        prev_ram_used: Vec::new(),
    })).lock().await;
    Json(info.clone())
}

#[get("/getdiskinfo")]
async fn getdiskinfo() -> Json<DISK_INFO> {

    let info = disk_info.get_or_init(|| Mutex::new(DISK_INFO{
        total_size: 0,
        used: 0.0,
        prev_write: Vec::new(),
        prev_read: Vec::new()
    })).lock().await;
    Json(info.clone())
}

#[get("/getosinfo")]
async fn getosinfo() -> Json<OS_INFO> {

    let info = os_info.get_or_init(|| Mutex::new(OS_INFO{
        name: System::name().unwrap_or_default(),
        kernel: System::kernel_version().unwrap_or_default(),
        uptime:System::uptime()/60/60/24,

    })).lock().await;
    Json(info.clone())
}

#[get("/static/<path..>")]
async fn static_handler(path: PathBuf) -> Option<NamedFile> {
    let mut path = Path::new(relative!("static")).join(path);
    NamedFile::open(path).await.ok()
}

#[rocket::launch]
async fn rocket() -> _ {

    init();

    let handle = tokio::spawn(async {
        loop {

            update().await;
            println!("Background task finished unexpectedly. Reload.");
        }

    });

    rocket::build()
        .manage(handle)
        .attach(Template::fairing())
        .configure(rocket::Config::figment().merge(("port", 8000)))
        .mount("/", routes![index])
        .mount("/", routes![getcpuinfo])
        .mount("/", routes![getraminfo])
        .mount("/", routes![getosinfo])
        .mount("/",routes![static_handler])
        .mount("/", routes![getdiskinfo])

}

async fn update() {
    loop {
        tokio::time::sleep(tokio::time::Duration::from_secs(10)).await;

        {
            let mut sys = sys_el.get_or_init(|| Mutex::new(System::new_all())).lock().await;
            sys.refresh_cpu_all();
            sys.refresh_memory();

            let cpus = sys.cpus();
            let (avg_freq, avg_usage) = if !cpus.is_empty() {
                let total_usage: f32 = cpus.iter().map(|c| c.cpu_usage()).sum();
                let total_freq: u64 = cpus.iter().map(|c| c.frequency()).sum();
                let avg_usage = (total_usage / cpus.len() as f32 * 10.0).round() / 10.0;
                (total_freq / cpus.len() as u64, avg_usage)
            } else {
                (0, 0.0)
            };

            let mut cpu = cpu_info.get_or_init(|| Mutex::new(CPU_INFO {
                name: "Unknown".to_string(),
                frequency: 0,
                cpu_count: 0,
                cpu_usage: 0.0,
                prev_cpus: vec![],
            })).lock().await;

            cpu.frequency = avg_freq;
            cpu.cpu_usage = avg_usage;
            cpu.cpu_count = cpus.len() as u16;
            if let Some(first) = cpus.first() {
                cpu.name = first.brand().trim().to_string();
            }

            cpu.prev_cpus.push(Pair { v1: chrono::Local::now().timestamp(), v2: avg_usage });
            if cpu.prev_cpus.len() > 10 {
                cpu.prev_cpus.remove(0);
            }

            // RAM
            let ram_used = if sys.total_memory() > 0 {
                ((sys.used_memory() as f32 / sys.total_memory() as f32) * 1000.0).round() / 10.0
            } else { 0.0 };

            let swap_used = if sys.total_swap() > 0 {
                ((sys.used_swap() as f32 / sys.total_swap() as f32) * 1000.0).round() / 10.0
            } else { 0.0 };

            let mut ram = ram_info.get_or_init(|| Mutex::new(RAM_INFO::default())).lock().await;
            ram.ram_size = (sys.total_memory() / 1_073_741_824) as u32;
            ram.swap_size = (sys.total_swap() / 1_073_741_824) as u32;
            ram.ram_used = ram_used;
            ram.swap_used = swap_used;
            ram.prev_ram_used.push(Pair { v1: chrono::Local::now().timestamp(), v2: ram_used });
            if ram.prev_ram_used.len() > 10 {
                ram.prev_ram_used.remove(0);
            }
        }

        {
            let mut disks = disk_el.get_or_init(|| Mutex::new(Disks::new_with_refreshed_list())).lock().await;

            disks.refresh(true);

            let mut total_size: u64 = 0;
            let mut avail: u64 = 0;
            let mut read_mb: i64 = 0;
            let mut write_mb: i64 = 0;

            for disk in disks.iter() {
                total_size = total_size.saturating_add(disk.total_space() / 1_073_741_824);
                avail = avail.saturating_add(disk.available_space() / 1_073_741_824);
                read_mb = read_mb.saturating_add((disk.usage().total_read_bytes / 1024 / 1024) as i64);
                write_mb = write_mb.saturating_add((disk.usage().total_written_bytes / 1024 / 1024) as i64);
            }

            let usage_percent = if total_size > 0 {
                ((total_size - avail) as f32 / total_size as f32) * 100.0
            } else {
                0.0
            };

            let mut disk_info_guard = disk_info.get_or_init(|| Mutex::new(DISK_INFO {
                total_size: 0,
                used: 0.0,
                prev_read: vec![],
                prev_write: vec![],
            })).lock().await;

            disk_info_guard.total_size = total_size.min(u32::MAX as u64) as u32;
            disk_info_guard.used = usage_percent;
            disk_info_guard.prev_read.push(Pair { v1: chrono::Local::now().timestamp(), v2: read_mb as i32 });
            disk_info_guard.prev_write.push(Pair { v1: chrono::Local::now().timestamp(), v2: write_mb as i32 });

            if disk_info_guard.prev_read.len() > 11 { disk_info_guard.prev_read.remove(0); }
            if disk_info_guard.prev_write.len() > 11 { disk_info_guard.prev_write.remove(0); }
        }
    }
}

fn execute_command(s: &str) -> String {
    #[cfg(target_os = "windows")]
    {
        let output = match Command::new("cmd")
            .args(&["/C", s])
            .creation_flags(0x08000000)
            .output()
        {
            Ok(r) => r.stdout,
            Err(_) => return System::name().unwrap_or("Undefined".to_string()),
        };

        let mut exec_result = match String::from_utf8(output) {
            Ok(res) => res,
            Err(_) => System::name().unwrap_or("Undefined".to_string()),
        };

        if exec_result.len() > 50 {
            exec_result = System::name().unwrap_or("Undefined".to_string());
        }

        exec_result
    }

    #[cfg(not(target_os = "windows"))]
    {
        let output = match Command::new("sh")
            .arg("-c")
            .arg(s)
            .output()
        {
            Ok(r) => r.stdout,
            Err(_) => return System::name().unwrap_or("Undefined".to_string()),
        };

        let exec_result = match String::from_utf8(output) {
            Ok(res) => res,
            Err(_) => System::name().unwrap_or("Undefined".to_string()),
        };

        exec_result
    }
}
