import { useState, useRef, useEffect, useCallback } from "react";
import { commands } from "../commands";
import type { AiChatResponse, MusicCard } from "../types";
import "./ChatBox.css";

interface Message {
  id: number;
  text: string;
  sender: "user" | "ai" | "system";
  time: Date;
  musicCard?: MusicCard;
}

// ── WAV Recording Helpers ──

function createWav(samples: Float32Array, sampleRate: number): Uint8Array {
  const numChannels = 1;
  const bitsPerSample = 16;
  const byteRate = sampleRate * numChannels * (bitsPerSample / 8);
  const blockAlign = numChannels * (bitsPerSample / 8);
  const dataSize = samples.length * (bitsPerSample / 8);
  const bufferSize = 44 + dataSize;

  const buffer = new ArrayBuffer(bufferSize);
  const view = new DataView(buffer);

  // RIFF header
  writeString(view, 0, "RIFF");
  view.setUint32(4, bufferSize - 8, true);
  writeString(view, 8, "WAVE");

  // fmt chunk
  writeString(view, 12, "fmt ");
  view.setUint32(16, 16, true);
  view.setUint16(20, 1, true);         // PCM
  view.setUint16(22, numChannels, true);
  view.setUint32(24, sampleRate, true);
  view.setUint32(28, byteRate, true);
  view.setUint16(32, blockAlign, true);
  view.setUint16(34, bitsPerSample, true);

  // data chunk
  writeString(view, 36, "data");
  view.setUint32(40, dataSize, true);

  // Write PCM samples
  let offset = 44;
  for (let i = 0; i < samples.length; i++) {
    const s = Math.max(-1, Math.min(1, samples[i]));
    view.setInt16(offset, s < 0 ? s * 0x8000 : s * 0x7fff, true);
    offset += 2;
  }

  return new Uint8Array(buffer);
}

function writeString(view: DataView, offset: number, str: string) {
  for (let i = 0; i < str.length; i++) {
    view.setUint8(offset + i, str.charCodeAt(i));
  }
}

export function ChatBox() {
  const [messages, setMessages] = useState<Message[]>([
    {
      id: 0,
      text: "欢迎回来，我的宝！🎵 Claudio 已经准备好了，想听什么歌跟我说～",
      sender: "ai",
      time: new Date(),
    },
  ]);
  const [input, setInput] = useState("");
  const [isLoading, setIsLoading] = useState(false);
  const [hasAiKey, setHasAiKey] = useState(false);
  const [hasSpeechKey, setHasSpeechKey] = useState(false);
  const [aiKeyInput, setAiKeyInput] = useState("");
  const [speechAkId, setSpeechAkId] = useState("");
  const [speechAkSecret, setSpeechAkSecret] = useState("");
  const [speechAppKey, setSpeechAppKey] = useState("");
  const [showConfig, setShowConfig] = useState<"ai" | "speech" | null>(null);
  const [isRecording, setIsRecording] = useState(false);
  const [isSpeaking, setIsSpeaking] = useState<number | null>(null); // message id being spoken
  const [inputMode, setInputMode] = useState<"voice" | "text">("voice"); // default voice mode
  const listRef = useRef<HTMLDivElement>(null);
  const inputRef = useRef<HTMLInputElement>(null);

  // Track which messages have been auto-spoken via TTS
  const autoSpokenRef = useRef<Set<number>>(new Set());

  // Recording refs
  const recordingRef = useRef<{
    stream: MediaStream | null;
    context: AudioContext | null;
    processor: ScriptProcessorNode | null;
    chunks: Float32Array[];
    source: MediaStreamAudioSourceNode | null;
  }>({
    stream: null,
    context: null,
    processor: null,
    chunks: [],
    source: null,
  });

  // Load configs on mount
  useEffect(() => {
    commands.getAiConfig().then((c) => setHasAiKey(!!c.api_key)).catch(() => {});
    commands.getSpeechConfig().then((c) => setHasSpeechKey(!!c.app_key)).catch(() => {});
  }, []);

  useEffect(() => {
    if (listRef.current) {
      listRef.current.scrollTop = listRef.current.scrollHeight;
    }
  }, [messages]);

  // Auto-play music card when AI sends one
  useEffect(() => {
    const last = messages[messages.length - 1];
    if (last?.musicCard?.track_index != null) {
      commands.play(last.musicCard.track_index).catch(console.error);
    }
  }, [messages]);

  // Auto-speak new AI replies via TTS (语音播报)
  useEffect(() => {
    const last = messages[messages.length - 1];
    if (last?.sender === "ai" && hasSpeechKey && !autoSpokenRef.current.has(last.id)) {
      autoSpokenRef.current.add(last.id);
      handleSpeak(last.text, last.id);
    }
  }, [messages, hasSpeechKey]);

  const addMessage = useCallback(
    (text: string, sender: "user" | "ai" | "system", musicCard?: MusicCard) => {
      setMessages((prev) => [
        ...prev,
        { id: Date.now(), text, sender, time: new Date(), musicCard },
      ]);
    },
    []
  );

  // ── API Key Settings ──

  const handleSetAiKey = async () => {
    const key = aiKeyInput.trim();
    if (!key) return;
    try {
      await commands.setAiApiKey(key);
      setHasAiKey(true);
      setShowConfig(null);
      setAiKeyInput("");
      addMessage("DeepSeek API Key 已设置成功！🎉", "system");
    } catch (e) {
      addMessage(`API Key 保存失败: ${e}`, "system");
    }
  };

  const handleSetSpeechKey = async () => {
    if (!speechAkId.trim() || !speechAkSecret.trim() || !speechAppKey.trim()) return;
    try {
      await commands.setSpeechConfig({
        access_key_id: speechAkId.trim(),
        access_key_secret: speechAkSecret.trim(),
        app_key: speechAppKey.trim(),
        voice: "zhixiaoxia",
        enabled: true,
      });
      setHasSpeechKey(true);
      setShowConfig(null);
      addMessage("ISI 语音配置已保存！现在可以用语音了 🎤", "system");
    } catch (e) {
      addMessage(`语音配置保存失败: ${e}`, "system");
    }
  };

  // ── TTS: Speak AI message ──

  const currentAudioRef = useRef<HTMLAudioElement | null>(null);

  const handleSpeak = async (text: string, msgId: number) => {
    // If already speaking this message, stop
    if (isSpeaking === msgId) {
      currentAudioRef.current?.pause();
      currentAudioRef.current = null;
      setIsSpeaking(null);
      return;
    }

    if (!hasSpeechKey) {
      setShowConfig("speech");
      addMessage("请先设置阿里云 ISI AccessKey 和 AppKey 开启语音功能 🔑", "system");
      return;
    }

    try {
      setIsSpeaking(msgId);
      const config = await commands.getSpeechConfig();
      const resp = await commands.synthesizeSpeech(text, config.voice);

      if (resp.audio_base64) {
        // Decode base64 to audio
        const binary = atob(resp.audio_base64);
        const bytes = new Uint8Array(binary.length);
        for (let i = 0; i < binary.length; i++) {
          bytes[i] = binary.charCodeAt(i);
        }
        const blob = new Blob([bytes], { type: "audio/wav" });
        const url = URL.createObjectURL(blob);

        const audio = new Audio(url);
        currentAudioRef.current = audio;

        audio.onended = () => {
          setIsSpeaking(null);
          URL.revokeObjectURL(url);
          currentAudioRef.current = null;
        };
        audio.onerror = () => {
          setIsSpeaking(null);
          URL.revokeObjectURL(url);
          currentAudioRef.current = null;
          addMessage("语音播放失败", "system");
        };

        await audio.play();
      }
    } catch (e) {
      setIsSpeaking(null);
      addMessage(`语音合成失败: ${e}`, "system");
    }
  };

  // ── ASR: Voice recording ──

  const handleToggleRecording = async () => {
    if (isRecording) {
      // Stop recording
      const rec = recordingRef.current;
      rec.processor?.disconnect();
      rec.source?.disconnect();
      rec.context?.close();
      rec.stream?.getTracks().forEach((t) => t.stop());

      // Build WAV from chunks
      if (rec.chunks.length === 0) {
        setIsRecording(false);
        return;
      }

      // Calculate total samples
      let totalLen = 0;
      for (const c of rec.chunks) totalLen += c.length;
      const allSamples = new Float32Array(totalLen);
      let offset = 0;
      for (const c of rec.chunks) {
        allSamples.set(c, offset);
        offset += c.length;
      }

      const sampleRate = rec.context?.sampleRate ?? 16000;
      const wavBytes = createWav(allSamples, sampleRate);

      setIsRecording(false);

      // Send to ASR
      try {
        addMessage("🎤 正在识别语音...", "system");
        const resp = await commands.recognizeSpeech(Array.from(wavBytes));
        // Always remove the "recognizing" message
        setMessages((prev) => prev.slice(0, -1));
        if (resp.text) {
          await handleSend(resp.text);
        }
      } catch (e) {
        // Remove the "recognizing" message first
        setMessages((prev) => prev.slice(0, -1));
        addMessage(`语音识别失败: ${e}`, "system");
      }

      recordingRef.current = { stream: null, context: null, processor: null, chunks: [], source: null };
    } else {
      // Start recording
      if (!hasSpeechKey) {
        setShowConfig("speech");
        addMessage("请先设置阿里云 ISI AccessKey 和 AppKey 开启语音功能 🔑", "system");
        return;
      }

      try {
        const stream = await navigator.mediaDevices.getUserMedia({ audio: true });
        const context = new AudioContext({ sampleRate: 16000 });
        const source = context.createMediaStreamSource(stream);

        const chunks: Float32Array[] = [];
        const processor = context.createScriptProcessor(4096, 1, 1);
        processor.onaudioprocess = (e) => {
          chunks.push(new Float32Array(e.inputBuffer.getChannelData(0)));
        };

        source.connect(processor);
        processor.connect(context.destination);

        recordingRef.current = { stream, context, processor, chunks, source };
        setIsRecording(true);
      } catch (e) {
        addMessage(`无法访问麦克风: ${e}`, "system");
      }
    }
  };

  // ── Chat ──

  const handleSend = async (overrideText?: string) => {
    const text = (overrideText ?? input).trim();
    if (!text || isLoading) return;

    if (!hasAiKey) {
      addMessage(text, "user");
      setInput("");
      setShowConfig("ai");
      addMessage("请先设置 DeepSeek API Key 🔑", "system");
      return;
    }

    if (text.startsWith("setkey:")) {
      const key = text.slice(7).trim();
      try {
        await commands.setAiApiKey(key);
        setHasAiKey(true);
        addMessage("setkey:****" + key.slice(-4), "user");
        addMessage("API Key 已更新！🎉", "system");
      } catch (e) {
        addMessage("setkey:****", "user");
        addMessage(`API Key 设置失败: ${e}`, "system");
      }
      setInput("");
      return;
    }

    setInput("");
    addMessage(text, "user");
    setIsLoading(true);

    try {
      const recentMessages = messages
        .filter((m) => m.sender === "user" || m.sender === "ai")
        .slice(-9)
        .map((m) => ({
          role: m.sender === "user" ? "user" : "assistant",
          content: m.text,
        }));

      const request = {
        messages: [
          ...recentMessages,
          { role: "user" as const, content: text },
        ],
      };

      const response: AiChatResponse = await commands.chatWithAI(request);
      addMessage(response.reply, "ai", response.music_card ?? undefined);
    } catch (e) {
      addMessage(`AI 回复出错: ${e}`, "system");
    } finally {
      setIsLoading(false);
      inputRef.current?.focus();
    }
  };

  const handleKeyDown = (e: React.KeyboardEvent) => {
    if (e.key === "Enter" && !e.shiftKey) {
      e.preventDefault();
      handleSend();
    }
  };

  return (
    <div className="chat-box">
      <div className="chat-messages" ref={listRef}>
        {messages.map((msg) => (
          <div key={msg.id} className={`chat-msg ${msg.sender}`}>
            {msg.sender === "ai" && <span className="chat-msg-avatar">♪</span>}
            <div className="chat-msg-content">
              <div className="chat-msg-text-row">
                <span className="chat-msg-text">{msg.text}</span>
                {msg.sender === "ai" && (
                  <button
                    className={`chat-speak-btn ${isSpeaking === msg.id ? "speaking" : ""}`}
                    onClick={() => handleSpeak(msg.text, msg.id)}
                    title={isSpeaking === msg.id ? "停止朗读" : "朗读回复"}
                  >
                    {isSpeaking === msg.id ? "■" : "♪"}
                  </button>
                )}
              </div>
              {msg.musicCard && (
                <div className="music-card">
                  <div className="music-card-icon">🎵</div>
                  <div className="music-card-info">
                    <span className="music-card-name">{msg.musicCard.track_name}</span>
                    <span className="music-card-artist">{msg.musicCard.artist}</span>
                  </div>
                  <button
                    className="music-card-play"
                    onClick={async () => {
                      if (msg.musicCard?.track_index != null) {
                        try {
                          await commands.play(msg.musicCard.track_index);
                        } catch (e) {
                          console.error("Play error:", e);
                        }
                      }
                    }}
                    title="播放"
                  >
                    ▶
                  </button>
                </div>
              )}
            </div>
          </div>
        ))}
        {isLoading && (
          <div className="chat-msg ai">
            <span className="chat-msg-avatar">♪</span>
            <div className="chat-msg-content">
              <span className="chat-msg-loading">
                <span className="dot-pulse">.</span>
                <span className="dot-pulse">.</span>
                <span className="dot-pulse">.</span>
              </span>
            </div>
          </div>
        )}
      </div>

      {/* API Key config rows */}
      {showConfig === "ai" && (
        <div className="chat-config-row">
          <span className="chat-config-label">DeepSeek API Key</span>
          <div className="chat-config-inputs">
            <input
              className="chat-input"
              type="password"
              placeholder="sk-..."
              value={aiKeyInput}
              onChange={(e) => setAiKeyInput(e.target.value)}
              onKeyDown={(e) => { if (e.key === "Enter") handleSetAiKey(); }}
            />
            <button className="chat-send" onClick={handleSetAiKey} disabled={!aiKeyInput.trim()}>
              设置
            </button>
          </div>
        </div>
      )}

      {showConfig === "speech" && (
        <div className="chat-config-row">
          <span className="chat-config-label">阿里云 ISI 语音配置</span>
          <div className="chat-config-inputs">
            <input
              className="chat-input"
              type="text"
              placeholder="AccessKey ID"
              value={speechAkId}
              onChange={(e) => setSpeechAkId(e.target.value)}
            />
          </div>
          <div className="chat-config-inputs">
            <input
              className="chat-input"
              type="password"
              placeholder="AccessKey Secret"
              value={speechAkSecret}
              onChange={(e) => setSpeechAkSecret(e.target.value)}
            />
          </div>
          <div className="chat-config-inputs">
            <input
              className="chat-input"
              type="text"
              placeholder="AppKey (ISI 控制台)"
              value={speechAppKey}
              onChange={(e) => setSpeechAppKey(e.target.value)}
              onKeyDown={(e) => { if (e.key === "Enter") handleSetSpeechKey(); }}
            />
            <button
              className="chat-send"
              onClick={handleSetSpeechKey}
              disabled={!speechAkId.trim() || !speechAkSecret.trim() || !speechAppKey.trim()}
            >
              保存
            </button>
          </div>
        </div>
      )}

      {/* Input row */}
      <div className="chat-input-row">
        {inputMode === "voice" && hasSpeechKey ? (
          <>
            <div className="chat-voice-area">
              <button
                className={`chat-voice-btn ${isRecording ? "recording" : ""}`}
                onClick={handleToggleRecording}
                title={isRecording ? "停止录音" : "点击说话"}
              >
                <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" strokeLinejoin="round" className="chat-mic-icon">
                  <path d="M12 1a3 3 0 0 0-3 3v8a3 3 0 0 0 6 0V4a3 3 0 0 0-3-3z"/>
                  <path d="M19 10v2a7 7 0 0 1-14 0v-2"/>
                  <line x1="12" y1="19" x2="12" y2="23"/>
                  <line x1="8" y1="23" x2="16" y2="23"/>
                </svg>
              </button>
            </div>
            <button
              className="chat-mode-btn"
              onClick={() => setInputMode("text")}
              title="键盘输入"
            >
              <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" strokeLinejoin="round" className="chat-mode-icon">
                <rect x="2" y="4" width="20" height="16" rx="2"/>
                <path d="M6 8h.01M10 8h.01M14 8h.01M18 8h.01M8 12h.01M12 12h.01M16 12h.01M6 16h12"/>
              </svg>
            </button>
          </>
        ) : (
          <>
            {hasSpeechKey && (
              <button
                className="chat-mode-btn"
                onClick={() => setInputMode("voice")}
                title="语音输入"
              >
                <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" strokeLinejoin="round" className="chat-mode-icon">
                  <path d="M12 1a3 3 0 0 0-3 3v8a3 3 0 0 0 6 0V4a3 3 0 0 0-3-3z"/>
                  <path d="M19 10v2a7 7 0 0 1-14 0v-2"/>
                  <line x1="12" y1="19" x2="12" y2="23"/>
                  <line x1="8" y1="23" x2="16" y2="23"/>
                </svg>
              </button>
            )}
            <input
              ref={inputRef}
              className="chat-input"
              type="text"
              placeholder={
                !hasAiKey
                  ? "先设置 DeepSeek API Key..."
                  : "输入消息..."
              }
              value={input}
              onChange={(e) => setInput(e.target.value)}
              onKeyDown={handleKeyDown}
              disabled={isLoading}
            />
            {!hasAiKey ? (
              <button className="chat-send" onClick={() => setShowConfig("ai")}>
                🔑
              </button>
            ) : !hasSpeechKey ? (
              <button className="chat-send" onClick={() => setShowConfig("speech")}>
                🔊
              </button>
            ) : (
              <button
                className="chat-send"
                onClick={() => handleSend()}
                disabled={!input.trim() || isLoading}
              >
                {isLoading ? "..." : "发送"}
              </button>
            )}
          </>
        )}
      </div>
    </div>
  );
}
