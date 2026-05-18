use std::process::Command;

pub fn get_context() -> String {
    let mut parts = Vec::new();

    // CPU usage
    if let Ok(out) = Command::new("top").args(["-l", "1", "-n", "0"]).output() {
        let text = String::from_utf8_lossy(&out.stdout);
        for line in text.lines() {
            if line.contains("CPU usage") {
                if let Some(idle_str) = line.split("idle").next() {
                    let idle_part = idle_str.trim().rsplit(',').next().unwrap_or("").trim();
                    if let Some(pct) = idle_part.strip_suffix('%') {
                        if let Ok(idle) = pct.trim().parse::<f32>() {
                            let used = 100.0 - idle;
                            parts.push(format!("CPU: {:.0}%", used));
                            if used > 80.0 {
                                parts.push("CPU WARNING: overloaded!".to_string());
                            }
                        }
                    }
                }
                break;
            }
        }
    }

    // Memory pressure
    if let Ok(out) = Command::new("memory_pressure").output() {
        let text = String::from_utf8_lossy(&out.stdout);
        for line in text.lines() {
            if line.contains("System-wide memory free percentage") {
                if let Some(pct_str) = line.split(':').nth(1) {
                    let pct_str = pct_str.trim().trim_end_matches('%').trim();
                    if let Ok(free) = pct_str.parse::<f32>() {
                        let used = 100.0 - free;
                        parts.push(format!("RAM: {:.0}% used", used));
                        if used > 80.0 {
                            parts.push("RAM WARNING: memory pressure high!".to_string());
                        }
                    }
                }
                break;
            }
        }
    }

    // Thermal state
    if let Ok(out) = Command::new("pmset").args(["-g", "therm"]).output() {
        let text = String::from_utf8_lossy(&out.stdout);
        if text.contains("CPU_Speed_Limit") {
            for line in text.lines() {
                if line.contains("CPU_Speed_Limit") {
                    if let Some(val) = line.split('=').nth(1) {
                        if let Ok(limit) = val.trim().parse::<u32>() {
                            if limit < 100 {
                                parts.push(format!("THERMAL: CPU throttled to {}%!", limit));
                            }
                        }
                    }
                }
            }
        }
    }

    if parts.is_empty() {
        "Performance: normal".to_string()
    } else {
        parts.join(", ")
    }
}
