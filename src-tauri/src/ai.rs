use std::sync::Mutex;

use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tauri::{AppHandle, Emitter};

use crate::audio::{self, AudioPlayer};
use crate::state::{AiChatRequest, AiChatResponse, AppState, MusicCard, PlaybackState, TrackInfo};

const DEEPSEEK_URL: &str = "https://api.deepseek.com/chat/completions";

// ── OpenAI-compatible API types ──

#[derive(Serialize)]
struct ChatRequest {
    model: String,
    messages: Vec<ChatMessage>,
    tools: Vec<ToolDef>,
    tool_choice: String,
}

#[derive(Clone, Serialize)]
struct ChatMessage {
    role: String,
    content: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    tool_calls: Option<Vec<ToolCall>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tool_call_id: Option<String>,
}

#[derive(Clone, Serialize, Deserialize)]
struct ToolCall {
    id: String,
    #[serde(rename = "type")]
    call_type: String,
    function: ToolCallFunction,
}

#[derive(Clone, Serialize, Deserialize)]
struct ToolCallFunction {
    name: String,
    arguments: String,
}

#[derive(Deserialize)]
struct ChatResponse {
    choices: Vec<Choice>,
}

#[derive(Deserialize)]
struct Choice {
    message: ResponseMessage,
}

#[derive(Deserialize)]
struct ResponseMessage {
    role: String,
    #[serde(default)]
    content: Option<String>,
    #[serde(default)]
    tool_calls: Option<Vec<ToolCall>>,
}

#[derive(Clone, Serialize)]
struct ToolDef {
    #[serde(rename = "type")]
    tool_type: String,
    function: ToolFunction,
}

#[derive(Clone, Serialize)]
struct ToolFunction {
    name: String,
    description: String,
    parameters: Value,
}

// ── Tool definitions ──

fn build_tools() -> Vec<ToolDef> {
    vec![
        ToolDef {
            tool_type: "function".to_string(),
            function: ToolFunction {
                name: "get_playlist".to_string(),
                description: "获取当前播放列表的所有歌曲".to_string(),
                parameters: serde_json::json!({ "type": "object", "properties": {} }),
            },
        },
        ToolDef {
            tool_type: "function".to_string(),
            function: ToolFunction {
                name: "get_current_track".to_string(),
                description: "获取当前正在播放的歌曲信息".to_string(),
                parameters: serde_json::json!({ "type": "object", "properties": {} }),
            },
        },
        ToolDef {
            tool_type: "function".to_string(),
            function: ToolFunction {
                name: "search_local_music".to_string(),
                description: "在本地播放列表中搜索匹配的歌曲，按歌名或歌手搜索".to_string(),
                parameters: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "query": {
                            "type": "string",
                            "description": "搜索关键词，可以是歌名或歌手名"
                        }
                    },
                    "required": ["query"]
                }),
            },
        },
        ToolDef {
            tool_type: "function".to_string(),
            function: ToolFunction {
                name: "play_track".to_string(),
                description: "播放指定索引的歌曲".to_string(),
                parameters: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "index": {
                            "type": "integer",
                            "description": "歌曲在播放列表中的索引（从0开始）"
                        }
                    },
                    "required": ["index"]
                }),
            },
        },
        ToolDef {
            tool_type: "function".to_string(),
            function: ToolFunction {
                name: "toggle_playback".to_string(),
                description: "切换播放/暂停状态".to_string(),
                parameters: serde_json::json!({ "type": "object", "properties": {} }),
            },
        },
        ToolDef {
            tool_type: "function".to_string(),
            function: ToolFunction {
                name: "next_track".to_string(),
                description: "播放下一首歌".to_string(),
                parameters: serde_json::json!({ "type": "object", "properties": {} }),
            },
        },
        ToolDef {
            tool_type: "function".to_string(),
            function: ToolFunction {
                name: "prev_track".to_string(),
                description: "播放上一首歌".to_string(),
                parameters: serde_json::json!({ "type": "object", "properties": {} }),
            },
        },
        ToolDef {
            tool_type: "function".to_string(),
            function: ToolFunction {
                name: "set_volume".to_string(),
                description: "调节音量大小，范围 0.0（静音）到 1.0（最大）".to_string(),
                parameters: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "volume": {
                            "type": "number",
                            "description": "音量值，0.0 到 1.0 之间"
                        }
                    },
                    "required": ["volume"]
                }),
            },
        },
        ToolDef {
            tool_type: "function".to_string(),
            function: ToolFunction {
                name: "stop_playback".to_string(),
                description: "停止播放".to_string(),
                parameters: serde_json::json!({ "type": "object", "properties": {} }),
            },
        },
    ]
}

// ── Tool execution ──

fn execute_tool(
    name: &str,
    args: &Value,
    state: &mut AppState,
    app: &AppHandle,
) -> Result<String, String> {
    match name {
        "get_playlist" => {
            let tracks: Vec<String> = state
                .playlist
                .iter()
                .enumerate()
                .map(|(i, t)| format!("{}. {} - {}", i, t.name, t.artist))
                .collect();
            if tracks.is_empty() {
                Ok("播放列表是空的，还没有添加任何歌曲。".to_string())
            } else {
                Ok(format!("当前播放列表：\n{}", tracks.join("\n")))
            }
        }

        "get_current_track" => {
            match &state.current_index {
                Some(idx) => {
                    if let Some(track) = state.playlist.get(*idx) {
                        let status = match state.playback_state {
                            PlaybackState::Playing => "▶ 正在播放",
                            PlaybackState::Paused => "⏸ 已暂停",
                            PlaybackState::Stopped => "⏹ 已停止",
                        };
                        Ok(format!("{}: {} - {}", status, track.name, track.artist))
                    } else {
                        Ok("当前没有播放任何歌曲。".to_string())
                    }
                }
                None => Ok("当前没有播放任何歌曲。".to_string()),
            }
        }

        "search_local_music" => {
            let query = args
                .get("query")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_lowercase();
            let results: Vec<(usize, &TrackInfo)> = state
                .playlist
                .iter()
                .enumerate()
                .filter(|(_, t)| {
                    t.name.to_lowercase().contains(&query) || t.artist.to_lowercase().contains(&query)
                })
                .collect();

            if results.is_empty() {
                Ok(format!("没有找到与\"{}\"匹配的歌曲。", query))
            } else {
                let list: Vec<String> = results
                    .iter()
                    .map(|(i, t)| format!("{} - {}（索引 {}）", t.name, t.artist, i))
                    .collect();
                Ok(format!("找到 {} 首匹配歌曲：\n{}", results.len(), list.join("\n")))
            }
        }

        "play_track" => {
            let idx = args
                .get("index")
                .and_then(|v| v.as_u64())
                .ok_or_else(|| "缺少 index 参数".to_string())? as usize;

            if idx >= state.playlist.len() {
                return Err(format!("索引 {} 超出播放列表范围（共 {} 首）", idx, state.playlist.len()));
            }

            let track = state.playlist[idx].clone();

            let mut player_lock = audio::get_audio_player().lock().map_err(|e| e.to_string())?;
            if player_lock.is_none() {
                *player_lock = Some(AudioPlayer::new()?);
            }
            let player = player_lock.as_mut().unwrap();
            let _duration = player.play(&track.path)?;

            state.current_index = Some(idx);
            state.playback_state = PlaybackState::Playing;
            player.set_volume(state.volume);

            drop(player_lock);

            let status = crate::commands::get_status(state);
            let _ = app.emit("playback-state-changed", &status);

            Ok(format!("正在播放：{} - {}", track.name, track.artist))
        }

        "toggle_playback" => {
            let mut player_lock = audio::get_audio_player().lock().map_err(|e| e.to_string())?;

            if let Some(player) = player_lock.as_mut() {
                match state.playback_state {
                    PlaybackState::Playing => {
                        player.pause();
                        state.playback_state = PlaybackState::Paused;
                        drop(player_lock);
                        let status = crate::commands::get_status(state);
                        let _ = app.emit("playback-state-changed", &status);
                        Ok("⏸ 已暂停播放".to_string())
                    }
                    PlaybackState::Paused => {
                        player.resume();
                        state.playback_state = PlaybackState::Playing;
                        drop(player_lock);
                        let status = crate::commands::get_status(state);
                        let _ = app.emit("playback-state-changed", &status);
                        Ok("▶ 已恢复播放".to_string())
                    }
                    PlaybackState::Stopped => {
                        drop(player_lock);
                        Ok("当前没有歌曲在播放，请先选择一首歌。".to_string())
                    }
                }
            } else {
                Ok("播放器尚未初始化。".to_string())
            }
        }

        "next_track" => {
            if state.playlist.is_empty() {
                return Ok("播放列表是空的。".to_string());
            }

            let next_idx = match state.current_index {
                Some(i) if i + 1 < state.playlist.len() => i + 1,
                Some(_) => 0,
                None => 0,
            };

            let track = state.playlist[next_idx].clone();
            state.current_index = Some(next_idx);
            state.playback_state = PlaybackState::Playing;

            if let Ok(mut player_lock) = audio::get_audio_player().lock() {
                if let Some(player) = player_lock.as_mut() {
                    let _ = player.play(&track.path);
                    player.set_volume(state.volume);
                }
            }

            let status = crate::commands::get_status(state);
            let _ = app.emit("playback-state-changed", &status);

            Ok(format!("切换到下一首：{} - {}", track.name, track.artist))
        }

        "prev_track" => {
            if state.playlist.is_empty() {
                return Ok("播放列表是空的。".to_string());
            }

            let prev_idx = match state.current_index {
                Some(i) if i > 0 => i - 1,
                Some(_) => state.playlist.len() - 1,
                None => 0,
            };

            let track = state.playlist[prev_idx].clone();
            state.current_index = Some(prev_idx);
            state.playback_state = PlaybackState::Playing;

            if let Ok(mut player_lock) = audio::get_audio_player().lock() {
                if let Some(player) = player_lock.as_mut() {
                    let _ = player.play(&track.path);
                    player.set_volume(state.volume);
                }
            }

            let status = crate::commands::get_status(state);
            let _ = app.emit("playback-state-changed", &status);

            Ok(format!("切换到上一首：{} - {}", track.name, track.artist))
        }

        "set_volume" => {
            let vol = args
                .get("volume")
                .and_then(|v| v.as_f64())
                .ok_or_else(|| "缺少 volume 参数".to_string())?
                .clamp(0.0, 1.0);

            state.volume = vol;
            if let Ok(player_lock) = audio::get_audio_player().lock() {
                if let Some(player) = player_lock.as_ref() {
                    player.set_volume(vol);
                }
            }

            let pct = (vol * 100.0).round() as u32;
            Ok(format!("音量已设置为 {}%", pct))
        }

        "stop_playback" => {
            if let Ok(mut player_lock) = audio::get_audio_player().lock() {
                if let Some(player) = player_lock.as_mut() {
                    player.stop();
                }
            }

            state.playback_state = PlaybackState::Stopped;
            let status = crate::commands::get_status(state);
            let _ = app.emit("playback-state-changed", &status);

            Ok("⏹ 已停止播放".to_string())
        }

        _ => Err(format!("未知工具: {}", name)),
    }
}

// ── Main chat handler ──

pub async fn chat_with_ai(
    request: AiChatRequest,
    api_key: String,
    model: String,
    state: &Mutex<AppState>,
    app: &AppHandle,
) -> Result<AiChatResponse, String> {
    if api_key.is_empty() {
        return Ok(AiChatResponse {
            reply: "请先设置你的 DeepSeek API Key，这样我才能和你聊天！在输入框中输入你的 API Key 即可。🔑".to_string(),
            music_card: None,
        });
    }

    let client = Client::new();
    let tools = build_tools();

    // Build messages for DeepSeek
    let mut chat_messages = vec![
        ChatMessage {
            role: "system".to_string(),
            content: "你是一个超有活力的音乐搭子，叫 Claudio，运行在 Claudio Desktop 音乐播放器里。\
                      你的任务就是像朋友一样跟用户聊天，帮ta推荐好听的歌、控制播放、扯扯日常。\
                      你可以用工具来看播放列表、搜歌、切歌、调音量什么的。\
                      注意几点：\
                      1. 说话要像真人一样，用口语，别整那些文绉绉的书面语。短句、自然，偶尔加个语气词。\
                      2. 根据用户的说话风格来调整，ta随意你也随意，ta正经你也正经点。\
                      3. 推荐音乐前先看看播放列表里有啥，别瞎推不存在的歌。\
                      4. 用户说要听什么歌，先搜一下再播放，别直接放。\
                      5. 每次操作后简单说一句你做了什么就行，别啰嗦。\
                      6. 适当用点音乐相关的emoji，但别刷屏。".to_string(),
            tool_calls: None,
            tool_call_id: None,
        },
    ];

    // Add user messages
    for msg in &request.messages {
        chat_messages.push(ChatMessage {
            role: msg.role.clone(),
            content: msg.content.clone(),
            tool_calls: None,
            tool_call_id: None,
        });
    }

    // Track whether play_track was called during tool execution
    let mut did_play = false;

    // Function calling loop (max 5 rounds)
    for _round in 0..5 {
        let chat_req = ChatRequest {
            model: model.clone(),
            messages: chat_messages.clone(),
            tools: tools.clone(),
            tool_choice: "auto".to_string(),
        };

        let resp: ChatResponse = client
            .post(DEEPSEEK_URL)
            .header("Authorization", format!("Bearer {}", api_key))
            .json(&chat_req)
            .send()
            .await
            .map_err(|e| format!("AI 请求失败: {}", e))?
            .json()
            .await
            .map_err(|e| format!("AI 响应解析失败: {}", e))?;

        let choice = resp
            .choices
            .into_iter()
            .next()
            .ok_or_else(|| "AI 返回了空响应".to_string())?;

        // Check for tool calls
        if let Some(tool_calls) = choice.message.tool_calls {
            let content = choice.message.content.unwrap_or_default();

            // Add assistant message with tool calls
            chat_messages.push(ChatMessage {
                role: "assistant".to_string(),
                content,
                tool_calls: Some(tool_calls.clone()),
                tool_call_id: None,
            });

            // Execute each tool call
            for tool_call in &tool_calls {
                let args: Value =
                    serde_json::from_str(&tool_call.function.arguments).unwrap_or(Value::Null);

                let result = {
                    let mut app_state = state.lock().map_err(|e| e.to_string())?;
                    let r = execute_tool(&tool_call.function.name, &args, &mut app_state, app);
                    if tool_call.function.name == "play_track" {
                        did_play = true;
                    }
                    r
                };

                let result_str = match result {
                    Ok(s) => s,
                    Err(e) => format!("错误: {}", e),
                };

                chat_messages.push(ChatMessage {
                    role: "tool".to_string(),
                    content: result_str,
                    tool_calls: None,
                    tool_call_id: Some(tool_call.id.clone()),
                });
            }

            // Continue the loop — send tool results back to AI
        } else {
            // No tool calls — this is the final text response
            let reply = choice.message.content.unwrap_or_default();

            // Build music card if the AI just played a track (regardless of reply wording)
            let music_card = if did_play {
                build_current_track_card(&state)
            } else {
                try_build_music_card(&reply, &state)
            };

            return Ok(AiChatResponse { reply, music_card });
        }
    }

    // Max rounds reached — still attach music card if we played something
    let music_card = if did_play {
        build_current_track_card(&state)
    } else {
        None
    };
    Ok(AiChatResponse {
        reply: "我已经处理了你的请求，但可能需要更具体的指令。请告诉我你还想做什么？🎵".to_string(),
        music_card,
    })
}

/// Build a music card for the currently playing track (unconditionally)
fn build_current_track_card(state: &Mutex<AppState>) -> Option<MusicCard> {
    let app_state = state.lock().ok()?;
    let idx = app_state.current_index?;
    let track = app_state.playlist.get(idx)?;
    Some(MusicCard {
        track_name: track.name.clone(),
        artist: track.artist.clone(),
        action: "play".to_string(),
        track_index: Some(idx),
    })
}

/// Check if the AI just played a track and build a music card for it
fn try_build_music_card(reply: &str, state: &Mutex<AppState>) -> Option<MusicCard> {
    if !reply.contains("正在播放") && !reply.contains("播放") {
        return None;
    }

    let app_state = state.lock().ok()?;
    let idx = app_state.current_index?;
    let track = app_state.playlist.get(idx)?;

    Some(MusicCard {
        track_name: track.name.clone(),
        artist: track.artist.clone(),
        action: "play".to_string(),
        track_index: Some(idx),
    })
}
