use tauri::Emitter;

#[tauri::command]
pub fn get_team_stats() -> Result<serde_json::Value, String> {
    let stats_path = crate::runtime::memory::amazon_data_dir().join("team_stats.json");
    let data = std::fs::read_to_string(&stats_path)
        .map_err(|_| "No team stats yet. Click Refresh to fetch.".to_string())?;
    serde_json::from_str(&data).map_err(|_| "Invalid stats data".to_string())
}

#[tauri::command]
pub fn get_gossip_history() -> Result<serde_json::Value, String> {
    let mem_path = crate::paths::memory_dir().join("cr_commented.json");
    let data = std::fs::read_to_string(&mem_path).unwrap_or_else(|_| r#"{"week_start":"","commented_ids":[]}"#.to_string());
    serde_json::from_str(&data).map_err(|_| "Invalid memory data".to_string())
}

#[tauri::command]
pub fn clear_gossip_memory() -> Result<(), String> {
    let mem_path = crate::paths::memory_dir().join("cr_commented.json");
    std::fs::write(&mem_path, r#"{"week_start":"","commented_ids":[]}"#)
        .map_err(|e| format!("Failed to clear: {}", e))
}

#[tauri::command]
pub fn refresh_team_stats() -> Result<serde_json::Value, String> {
    let amazon_dir = crate::runtime::memory::amazon_data_dir();
    let config_path = amazon_dir.join("config.json");

    let user_alias = std::fs::read_to_string(&config_path).ok()
        .and_then(|d| serde_json::from_str::<serde_json::Value>(&d).ok())
        .and_then(|c| c["user_alias"].as_str().map(|s| s.to_string()))
        .unwrap_or_default();

    if user_alias.is_empty() {
        return Err("No alias configured".to_string());
    }

    let days = std::fs::read_to_string(&config_path).ok()
        .and_then(|d| serde_json::from_str::<serde_json::Value>(&d).ok())
        .and_then(|c| c["cr_days_range"].as_u64())
        .unwrap_or(14);
    let days_str = days.to_string();

    crate::runtime::mcp_runner::call_mcp("amazon-internal", &["get-stats", "--alias", &user_alias, "--days", &days_str])
        .map_err(|e| format!("MCP failed: {}", e))
}

#[tauri::command]
pub async fn trigger_cr_comment(app: tauri::AppHandle) -> Result<String, String> {
    let amazon_dir = crate::runtime::memory::amazon_data_dir();
    let commits_path = amazon_dir.join("commits.json");
    let config_path = amazon_dir.join("config.json");

    let user_alias = std::fs::read_to_string(&config_path).ok()
        .and_then(|d| serde_json::from_str::<serde_json::Value>(&d).ok())
        .and_then(|c| c["user_alias"].as_str().map(|s| s.to_string()))
        .unwrap_or_default();

    if user_alias.is_empty() {
        return Err("No alias configured in Amazon tab".to_string());
    }

    let cr_days_range = std::fs::read_to_string(&config_path).ok()
        .and_then(|d| serde_json::from_str::<serde_json::Value>(&d).ok())
        .and_then(|c| c["cr_days_range"].as_u64())
        .unwrap_or(14);

    let needs_refresh = if !commits_path.exists() {
        true
    } else {
        std::fs::metadata(&commits_path).ok()
            .and_then(|m| m.modified().ok())
            .map(|t| t.elapsed().unwrap_or_default().as_secs() > 14400)
            .unwrap_or(true)
    };

    let fetch_commits = |days: u64| -> bool {
        let team_result = crate::runtime::mcp_runner::call_mcp("amazon-internal", &["get-team", "--alias", &user_alias]);
        if let Ok(team) = team_result {
            if let Some(teammates) = team["teammates"].as_array() {
                let mut all_aliases: Vec<String> = teammates.iter()
                    .filter_map(|t| t.as_str().map(|s| s.to_string()))
                    .collect();
                all_aliases.push(user_alias.clone());
                let aliases_str = all_aliases.join(",");
                let days_str = days.to_string();
                let _ = crate::runtime::mcp_runner::call_mcp("amazon-internal", &["get-commits", "--aliases", &aliases_str, "--days", &days_str]);
                return true;
            }
        }
        false
    };

    if needs_refresh {
        fetch_commits(cr_days_range);
    }

    let commits_data = std::fs::read_to_string(&commits_path)
        .map_err(|_| "No commits cached. Check Midway cookie.".to_string())?;
    let commits: Vec<serde_json::Value> = serde_json::from_str(&commits_data)
        .map_err(|_| "Invalid commits cache".to_string())?;

    if commits.is_empty() {
        return Err("No commits found for your team".to_string());
    }

    let commented: Vec<String> = crate::runtime::memory::get_commented_ids();
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH).unwrap().subsec_nanos();
    let available: Vec<&serde_json::Value> = commits.iter()
        .filter(|c| {
            let id = format!("{}:{}", c["author"].as_str().unwrap_or(""), c["title"].as_str().unwrap_or(""));
            !commented.contains(&id)
        })
        .collect();

    if available.is_empty() {
        let expanded = (cr_days_range * 2).min(90);
        if expanded > cr_days_range {
            fetch_commits(expanded);
            if let Ok(data) = std::fs::read_to_string(&commits_path) {
                if let Ok(expanded_commits) = serde_json::from_str::<Vec<serde_json::Value>>(&data) {
                    let expanded_available: Vec<&serde_json::Value> = expanded_commits.iter()
                        .filter(|c| {
                            let id = format!("{}:{}", c["author"].as_str().unwrap_or(""), c["title"].as_str().unwrap_or(""));
                            !commented.contains(&id)
                        })
                        .collect();
                    if !expanded_available.is_empty() {
                        let teammate_exp: Vec<&&serde_json::Value> = expanded_available.iter()
                            .filter(|c| c["author"].as_str().unwrap_or("") != user_alias)
                            .collect();
                        let commit = if !teammate_exp.is_empty() {
                            *teammate_exp[(nanos as usize) % teammate_exp.len()]
                        } else {
                            expanded_available[(nanos as usize) % expanded_available.len()]
                        };
                        let author = commit["author"].as_str().unwrap_or("");
                        let title = commit["title"].as_str().unwrap_or("");
                        let package = commit["package"].as_str().unwrap_or("");
                        let changes = commit["changes"].as_u64().unwrap_or(0);
                        let cfg = crate::config::load_config();
                        let lang_hint = match cfg.language_mix.as_str() {
                            "chinese" => "Reply ONLY in Chinese.",
                            "english" => "Reply ONLY in English.",
                            _ => "Mix Chinese and English naturally.",
                        };
                        let prompt = format!(
                            "You are a snarky cat reviewing your owner's teammate's code.\nTeammate: \"{}\"\nPackage: \"{}\"\nCommit title: \"{}\"\nFiles changed: {}\n{}\nGenerate ONE short funny/snarky comment (under 50 chars). MUST include the teammate's name. Reference THIS SPECIFIC commit title \"{}\" — do NOT mention unrelated topics. Start with 😼 {}.",
                            author, package, title, changes, lang_hint, title, author
                        );
                        let config = crate::config::load_config();
                        let message = crate::ai::send_to_ai(&config, &prompt, 80, 0.95).await
                            .map_err(|e| format!("AI failed: {}", e))?;
                        let _ = app.emit("module-reaction", serde_json::json!({
                            "module_id": "amazon-internal", "message": message, "priority": 7,
                        }));
                        crate::runtime::memory::mark_commit_commented(&format!("{}:{}", author, title));
                        return Ok(message);
                    }
                }
            }
        }
        return Err("team很安静...等新代码中 🐱".to_string());
    }

    // Load team stats (refresh if stale)
    let stats_path = amazon_dir.join("team_stats.json");
    let stats_stale = if !stats_path.exists() {
        true
    } else {
        std::fs::metadata(&stats_path).ok()
            .and_then(|m| m.modified().ok())
            .map(|t| t.elapsed().unwrap_or_default().as_secs() > 14400)
            .unwrap_or(true)
    };
    if stats_stale {
        let days_str = cr_days_range.to_string();
        let _ = crate::runtime::mcp_runner::call_mcp("amazon-internal", &["get-stats", "--alias", &user_alias, "--days", &days_str]);
    }
    let team_stats: Option<serde_json::Value> = std::fs::read_to_string(&stats_path).ok()
        .and_then(|d| serde_json::from_str(&d).ok());

    let teammate_available: Vec<&&serde_json::Value> = available.iter()
        .filter(|c| c["author"].as_str().unwrap_or("") != user_alias)
        .collect();

    let commit = if !teammate_available.is_empty() {
        let mut author_comment_counts: std::collections::HashMap<&str, usize> = std::collections::HashMap::new();
        for id in &commented {
            if let Some(a) = id.split(':').next() {
                *author_comment_counts.entry(a).or_insert(0) += 1;
            }
        }
        let min_count = teammate_available.iter()
            .map(|c| {
                let a = c["author"].as_str().unwrap_or("");
                author_comment_counts.get(a).copied().unwrap_or(0)
            })
            .min()
            .unwrap_or(0);
        let least_commented: Vec<&&serde_json::Value> = teammate_available.iter()
            .filter(|c| {
                let a = c["author"].as_str().unwrap_or("");
                author_comment_counts.get(a).copied().unwrap_or(0) == min_count
            })
            .copied()
            .collect();
        *least_commented[(nanos as usize) % least_commented.len()]
    } else {
        available[(nanos as usize) % available.len()]
    };
    let author = commit["author"].as_str().unwrap_or("");
    let title = commit["title"].as_str().unwrap_or("");
    let package = commit["package"].as_str().unwrap_or("");
    let changes = commit["changes"].as_u64().unwrap_or(0);

    let nicknames: std::collections::HashMap<String, String> = std::fs::read_to_string(&config_path).ok()
        .and_then(|d| serde_json::from_str::<serde_json::Value>(&d).ok())
        .and_then(|c| c["nicknames"].as_object().map(|m| {
            m.iter().map(|(k, v)| (k.clone(), v.as_str().unwrap_or("").to_string())).collect()
        }))
        .unwrap_or_default();
    let display_name = nicknames.get(author).filter(|n| !n.is_empty()).map(|n| n.as_str()).unwrap_or(author);

    let config = crate::config::load_config();
    let lang_hint = match config.language_mix.as_str() {
        "chinese" => "Reply ONLY in Chinese.",
        "english" => "Reply ONLY in English.",
        _ => if (nanos % 2) == 0 { "Mix Chinese and English naturally." } else { "Use Chinese with some English tech words." },
    };
    let sass_hint = match config.sass_level {
        1 => "Be very gentle and supportive.",
        2 => "Be mildly teasing but mostly kind.",
        4 => "Be quite snarky and roast them a bit.",
        5 => "Be MAXIMUM savage. Roast them hard but funny.",
        _ => "",
    };

    // Choose gossip mode: 40% roast, 30% comparison, 30% content reaction
    let gossip_mode = match nanos % 10 {
        0..=3 => "roast",
        4..=6 => "comparison",
        _ => "content",
    };

    let prompt = if author == user_alias {
        format!(
            "You are an encouraging cat praising your owner's code.\nPackage: \"{}\"\nCommit: \"{}\"\nFiles changed: {}\n{}\nGenerate ONE short encouraging message (under 50 chars). Your comment MUST be about this specific commit: \"{}\". Do NOT reference other work. Start with 😸",
            package, title, changes, lang_hint, title
        )
    } else {
        let stats_context = team_stats.as_ref().map(|ts| {
            let teammate_stats = &ts["teammates"][author];
            let comparisons = &ts["comparisons"];
            format!(
                "\nTEAM STATS:\n- {}: {} CRs, {} commits, streak {} days, avg {} files/CR\n- Most active: {}\n- Quietest: {}\n",
                display_name,
                teammate_stats["total_crs"].as_u64().unwrap_or(0),
                teammate_stats["total_commits"].as_u64().unwrap_or(0),
                teammate_stats["streak_days"].as_u64().unwrap_or(0),
                teammate_stats["avg_files_per_cr"].as_f64().unwrap_or(0.0),
                comparisons["most_crs"].as_str().unwrap_or("?"),
                comparisons["quietest"].as_str().unwrap_or("?"),
            )
        }).unwrap_or_default();

        match gossip_mode {
            "comparison" => format!(
                "You are a snarky cat comparing your owner's teammates' coding activity.\n{}\n{} {}\nGenerate ONE short comparison quip (under 50 chars). Compare activity levels. MUST mention \"{}\". Start with 😼",
                stats_context, lang_hint, sass_hint, display_name
            ),
            "content" => format!(
                "You are a snarky cat reacting to WHAT a teammate is working on.\nTeammate: \"{}\"\nThey're working on: \"{}\"\nPackage: \"{}\"\nFiles changed: {}\n{} {}\nReact to the TOPIC of their work (not just that they committed). Be specific about what \"{}\" means. MUST mention \"{}\". Under 50 chars. Start with 😼",
                display_name, title, package, changes, lang_hint, sass_hint, title, display_name
            ),
            _ => format!(
                "You are a snarky cat roasting your owner's teammate's coding HABITS.\nTeammate: \"{}\"\nPackage: \"{}\"\nCommit: \"{}\"\nFiles changed: {}\n{}{} {}\nRoast their coding habits (PR size, frequency, package choice). MUST use the name \"{}\". Under 50 chars. Start with 😼 {}.",
                display_name, package, title, changes, stats_context, lang_hint, sass_hint, display_name, display_name
            ),
        }
    };

    let config = crate::config::load_config();
    let message = crate::ai::send_to_ai(&config, &prompt, 80, 0.95).await
        .map_err(|e| format!("AI failed: {}", e))?;

    let _ = app.emit("module-reaction", serde_json::json!({
        "module_id": "amazon-internal",
        "message": message,
        "priority": 7,
    }));

    let commit_id = format!("{}:{}", author, title);
    crate::runtime::memory::mark_commit_commented(&commit_id);

    Ok(message)
}

#[tauri::command]
pub async fn trigger_targeted_gossip(app: tauri::AppHandle, target_alias: String) -> Result<String, String> {
    let amazon_dir = crate::runtime::memory::amazon_data_dir();
    let config_path = amazon_dir.join("config.json");
    let commits_path = amazon_dir.join("commits.json");
    let stats_path = amazon_dir.join("team_stats.json");

    let user_alias = std::fs::read_to_string(&config_path).ok()
        .and_then(|d| serde_json::from_str::<serde_json::Value>(&d).ok())
        .and_then(|c| c["user_alias"].as_str().map(|s| s.to_string()))
        .unwrap_or_default();

    if user_alias.is_empty() {
        return Err("No alias configured".to_string());
    }

    // Auto-fetch commits if not cached
    if !commits_path.exists() {
        let cr_days = std::fs::read_to_string(&config_path).ok()
            .and_then(|d| serde_json::from_str::<serde_json::Value>(&d).ok())
            .and_then(|c| c["cr_days_range"].as_u64())
            .unwrap_or(14);
        let team_result = crate::runtime::mcp_runner::call_mcp("amazon-internal", &["get-team", "--alias", &user_alias]);
        if let Ok(team) = team_result {
            if let Some(teammates) = team["teammates"].as_array() {
                let mut all_aliases: Vec<String> = teammates.iter()
                    .filter_map(|t| t.as_str().map(|s| s.to_string()))
                    .collect();
                all_aliases.push(user_alias.clone());
                let aliases_str = all_aliases.join(",");
                let days_str = cr_days.to_string();
                let _ = crate::runtime::mcp_runner::call_mcp("amazon-internal", &["get-commits", "--aliases", &aliases_str, "--days", &days_str]);
            }
        }
    }

    let commits_data = std::fs::read_to_string(&commits_path)
        .map_err(|_| "Failed to fetch commits. Check Midway session.".to_string())?;
    let commits: Vec<serde_json::Value> = serde_json::from_str(&commits_data)
        .map_err(|_| "Invalid commits cache".to_string())?;

    let target_commits: Vec<&serde_json::Value> = commits.iter()
        .filter(|c| c["author"].as_str().unwrap_or("") == target_alias)
        .collect();

    if target_commits.is_empty() {
        let _ = app.emit("dev-log", serde_json::json!({
            "tag": "JUDGE", "tag_class": "skip", "message": format!("{} has no recent commits", target_alias)
        }));
        return Err(format!("{}最近没有commit...在摸鱼吧 🐟", target_alias));
    }

    let nicknames: std::collections::HashMap<String, String> = std::fs::read_to_string(&config_path).ok()
        .and_then(|d| serde_json::from_str::<serde_json::Value>(&d).ok())
        .and_then(|c| c["nicknames"].as_object().map(|m| {
            m.iter().map(|(k, v)| (k.clone(), v.as_str().unwrap_or("").to_string())).collect()
        }))
        .unwrap_or_default();
    let display_name = nicknames.get(&target_alias).filter(|n| !n.is_empty()).map(|n| n.as_str()).unwrap_or(&target_alias);

    let team_stats: Option<serde_json::Value> = std::fs::read_to_string(&stats_path).ok()
        .and_then(|d| serde_json::from_str(&d).ok());

    let stats_context = team_stats.as_ref().map(|ts| {
        let s = &ts["teammates"][target_alias.as_str()];
        format!(
            "STATS for {}:\n- {} CRs, {} commits total\n- Streak: {} days\n- Avg {} files per CR\n- Biggest CR: {} files (\"{}\")\n- Packages: {:?}\n- Most active day: {}\n",
            display_name,
            s["total_crs"].as_u64().unwrap_or(0),
            s["total_commits"].as_u64().unwrap_or(0),
            s["streak_days"].as_u64().unwrap_or(0),
            s["avg_files_per_cr"].as_f64().unwrap_or(0.0),
            s["largest_cr_files"].as_u64().unwrap_or(0),
            s["largest_cr_title"].as_str().unwrap_or(""),
            s["packages_touched"].as_array().map(|a| a.iter().filter_map(|v| v.as_str()).collect::<Vec<_>>()).unwrap_or_default(),
            s["most_active_day"].as_str().unwrap_or(""),
        )
    }).unwrap_or_default();

    let recent_titles: String = target_commits.iter().take(3)
        .filter_map(|c| c["title"].as_str())
        .map(|t| format!("- \"{}\"", t))
        .collect::<Vec<_>>()
        .join("\n");

    let config = crate::config::load_config();
    let lang_hint = match config.language_mix.as_str() {
        "chinese" => "Reply ONLY in Chinese.",
        "english" => "Reply ONLY in English.",
        _ => "Mix Chinese and English naturally.",
    };
    let sass_hint = match config.sass_level {
        1 => "Be gentle but observant.",
        2 => "Be mildly teasing.",
        4 => "Be quite snarky.",
        5 => "Be MAXIMUM savage roast mode.",
        _ => "Be playfully snarky.",
    };

    let prompt = format!(
        "You are a snarky cat giving a FULL ASSESSMENT of a teammate's recent coding activity.\n\nTarget: \"{}\" (alias: {})\n{}\nRecent commits:\n{}\n\n{} {}\n\nGive a SHORT (under 60 chars) brutally honest assessment of this person's coding habits. Mention their name. Comment on their patterns (speed, PR size, package focus). Start with 😼",
        display_name, target_alias, stats_context, recent_titles, lang_hint, sass_hint
    );

    let _ = app.emit("dev-log", serde_json::json!({
        "tag": "JUDGE", "tag_class": "reaction", "message": format!("Judging {}...", display_name)
    }));

    let message = crate::ai::send_to_ai(&config, &prompt, 100, 0.95).await
        .map_err(|e| format!("AI failed: {}", e))?;

    let _ = app.emit("dev-log", serde_json::json!({
        "tag": "JUDGE", "tag_class": "reaction", "message": format!("{}: {}", display_name, message)
    }));

    let _ = app.emit("module-reaction", serde_json::json!({
        "module_id": "amazon-internal",
        "message": message,
        "priority": 8,
    }));

    Ok(message)
}
