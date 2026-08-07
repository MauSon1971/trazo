use log::{debug, warn};
use serde::de::{self, Visitor};
use serde::{Deserialize, Deserializer, Serialize};
use specta::Type;
use std::collections::HashMap;
use std::fmt;
use tauri::AppHandle;
use tauri_plugin_store::StoreExt;

pub const APPLE_INTELLIGENCE_PROVIDER_ID: &str = "apple_intelligence";
pub const APPLE_INTELLIGENCE_DEFAULT_MODEL_ID: &str = "Apple Intelligence";

#[derive(Serialize, Debug, Clone, Copy, PartialEq, Eq, Type)]
#[serde(rename_all = "lowercase")]
pub enum LogLevel {
    Trace,
    Debug,
    Info,
    Warn,
    Error,
}

// Custom deserializer to handle both old numeric format (1-5) and new string format ("trace", "debug", etc.)
impl<'de> Deserialize<'de> for LogLevel {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct LogLevelVisitor;

        impl<'de> Visitor<'de> for LogLevelVisitor {
            type Value = LogLevel;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("a string or integer representing log level")
            }

            fn visit_str<E: de::Error>(self, value: &str) -> Result<LogLevel, E> {
                match value.to_lowercase().as_str() {
                    "trace" => Ok(LogLevel::Trace),
                    "debug" => Ok(LogLevel::Debug),
                    "info" => Ok(LogLevel::Info),
                    "warn" => Ok(LogLevel::Warn),
                    "error" => Ok(LogLevel::Error),
                    _ => Err(E::unknown_variant(
                        value,
                        &["trace", "debug", "info", "warn", "error"],
                    )),
                }
            }

            fn visit_u64<E: de::Error>(self, value: u64) -> Result<LogLevel, E> {
                match value {
                    1 => Ok(LogLevel::Trace),
                    2 => Ok(LogLevel::Debug),
                    3 => Ok(LogLevel::Info),
                    4 => Ok(LogLevel::Warn),
                    5 => Ok(LogLevel::Error),
                    _ => Err(E::invalid_value(de::Unexpected::Unsigned(value), &"1-5")),
                }
            }
        }

        deserializer.deserialize_any(LogLevelVisitor)
    }
}

impl From<LogLevel> for tauri_plugin_log::LogLevel {
    fn from(level: LogLevel) -> Self {
        match level {
            LogLevel::Trace => tauri_plugin_log::LogLevel::Trace,
            LogLevel::Debug => tauri_plugin_log::LogLevel::Debug,
            LogLevel::Info => tauri_plugin_log::LogLevel::Info,
            LogLevel::Warn => tauri_plugin_log::LogLevel::Warn,
            LogLevel::Error => tauri_plugin_log::LogLevel::Error,
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, Type)]
pub struct ShortcutBinding {
    pub id: String,
    pub name: String,
    pub description: String,
    pub default_binding: String,
    pub current_binding: String,
}

/// One dictionary expansion rule: `from` (what you say, e.g. "pq") becomes
/// `to` (what gets written, e.g. "porque"). Matching is case-insensitive and
/// whole-word; see [`crate::audio_toolkit::text::apply_custom_replacements`].
#[derive(Serialize, Deserialize, Debug, Clone, Type)]
pub struct CustomReplacement {
    pub from: String,
    pub to: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, Type)]
pub struct LLMPrompt {
    pub id: String,
    pub name: String,
    pub prompt: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, Type)]
pub struct PostProcessProvider {
    pub id: String,
    pub label: String,
    pub base_url: String,
    #[serde(default)]
    pub allow_base_url_edit: bool,
    #[serde(default)]
    pub models_endpoint: Option<String>,
    #[serde(default)]
    pub supports_structured_output: bool,
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, Type)]
#[serde(rename_all = "lowercase")]
pub enum OverlayPosition {
    Top,
    // `none` is retired: overlay visibility is owned by `OverlayStyle` now. The
    // alias keeps legacy stores (`"overlay_position": "none"`) deserializing
    // instead of failing the whole load; the one-time overlay migration reads the
    // raw stored string to recover the old "hidden" intent as `OverlayStyle::None`.
    #[serde(alias = "none")]
    Bottom,
}

/// Which recording overlay to display. `Minimal` and `Live` share one base
/// (the pill); `Live` grows into the panel that shows live transcription text.
/// `None` hides the overlay entirely. Decoupled from whether the model runs in
/// streaming mode (that is driven purely by model capability).
#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, Type)]
#[serde(rename_all = "lowercase")]
pub enum OverlayStyle {
    None,
    Minimal,
    Live,
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, Type, Default)]
#[serde(rename_all = "snake_case")]
pub enum ModelUnloadTimeout {
    Never,
    Immediately,
    Min2,
    #[default]
    Min5,
    Min10,
    Min15,
    Hour1,
    Sec15, // Debug mode only
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, Type)]
#[serde(rename_all = "snake_case")]
pub enum PasteMethod {
    CtrlV,
    Direct,
    None,
    ShiftInsert,
    CtrlShiftV,
    ExternalScript,
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, Type, Default)]
#[serde(rename_all = "snake_case")]
pub enum ClipboardHandling {
    DontModify,
    // Default so a silent paste failure (which cannot be detected) never loses
    // a dictation: the transcript always survives on the clipboard.
    #[default]
    CopyToClipboard,
}

/// Tratamiento gramatical que usa el formalizador de correo.
///
/// Es un ajuste fijo y no algo que el LLM infiera: en español tú/usted cambia
/// cada verbo del mensaje, así que inferirlo haría que el mismo dictado saliera
/// distinto en dos intentos y no se pudiera cubrir con tests.
#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, Type, Default)]
#[serde(rename_all = "snake_case")]
pub enum FormalityTreatment {
    #[default]
    Tu,
    Usted,
}

impl FormalityTreatment {
    /// La palabra que se inyecta en `${tratamiento}`.
    pub fn as_prompt_word(self) -> &'static str {
        match self {
            FormalityTreatment::Tu => "tú",
            FormalityTreatment::Usted => "usted",
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, Type, Default)]
#[serde(rename_all = "snake_case")]
pub enum AutoSubmitKey {
    #[default]
    Enter,
    CtrlEnter,
    CmdEnter,
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, Type)]
#[serde(rename_all = "snake_case")]
pub enum RecordingRetentionPeriod {
    Never,
    PreserveLimit,
    Days3,
    Weeks2,
    Months3,
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, Type)]
#[serde(rename_all = "snake_case")]
pub enum KeyboardImplementation {
    Tauri,
    HandyKeys,
}

impl Default for KeyboardImplementation {
    fn default() -> Self {
        #[cfg(target_os = "linux")]
        return KeyboardImplementation::Tauri;
        #[cfg(not(target_os = "linux"))]
        return KeyboardImplementation::HandyKeys;
    }
}

impl Default for PasteMethod {
    fn default() -> Self {
        // Default to CtrlV for macOS and Windows, Direct for Linux
        #[cfg(target_os = "linux")]
        return PasteMethod::Direct;
        #[cfg(not(target_os = "linux"))]
        return PasteMethod::CtrlV;
    }
}

impl ModelUnloadTimeout {
    pub fn to_minutes(self) -> Option<u64> {
        match self {
            ModelUnloadTimeout::Never => None,
            ModelUnloadTimeout::Immediately => Some(0), // Special case for immediate unloading
            ModelUnloadTimeout::Min2 => Some(2),
            ModelUnloadTimeout::Min5 => Some(5),
            ModelUnloadTimeout::Min10 => Some(10),
            ModelUnloadTimeout::Min15 => Some(15),
            ModelUnloadTimeout::Hour1 => Some(60),
            ModelUnloadTimeout::Sec15 => Some(0), // Special case for debug - handled separately
        }
    }

    pub fn to_seconds(self) -> Option<u64> {
        match self {
            ModelUnloadTimeout::Never => None,
            ModelUnloadTimeout::Immediately => Some(0), // Special case for immediate unloading
            ModelUnloadTimeout::Sec15 => Some(15),
            _ => self.to_minutes().map(|m| m * 60),
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, Type)]
#[serde(rename_all = "snake_case")]
pub enum SoundTheme {
    Marimba,
    Pop,
    Custom,
}

impl SoundTheme {
    fn as_str(&self) -> &'static str {
        match self {
            SoundTheme::Marimba => "marimba",
            SoundTheme::Pop => "pop",
            SoundTheme::Custom => "custom",
        }
    }

    pub fn to_start_path(self) -> String {
        format!("resources/{}_start.wav", self.as_str())
    }

    pub fn to_stop_path(self) -> String {
        format!("resources/{}_stop.wav", self.as_str())
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, Type, Default)]
#[serde(rename_all = "snake_case")]
pub enum TypingTool {
    #[default]
    Auto,
    Wtype,
    Kwtype,
    Dotool,
    Ydotool,
    Xdotool,
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, Type, Default)]
#[serde(rename_all = "snake_case")]
pub enum TranscribeAcceleratorSetting {
    #[default]
    Auto,
    Cpu,
    Gpu,
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, Type, Default)]
#[serde(rename_all = "snake_case")]
pub enum OrtAcceleratorSetting {
    #[default]
    Auto,
    Cpu,
    Cuda,
    #[serde(rename = "directml")]
    DirectMl,
    Rocm,
}

#[derive(Clone, Serialize, Deserialize, Type)]
#[serde(transparent)]
pub(crate) struct SecretMap(HashMap<String, String>);

impl fmt::Debug for SecretMap {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let redacted: HashMap<&String, &str> = self
            .0
            .iter()
            .map(|(k, v)| (k, if v.is_empty() { "" } else { "[REDACTED]" }))
            .collect();
        redacted.fmt(f)
    }
}

impl std::ops::Deref for SecretMap {
    type Target = HashMap<String, String>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl std::ops::DerefMut for SecretMap {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

/* still handy for composing the initial JSON in the store ------------- */
#[derive(Serialize, Deserialize, Debug, Clone, Type)]
pub struct AppSettings {
    /// Internal settings schema marker for one-time migrations. Fresh installs
    /// start at the current version; existing stores missing this key are
    /// treated as version 0 and migrated forward.
    #[serde(default = "default_settings_schema_version")]
    pub settings_schema_version: u32,
    pub bindings: HashMap<String, ShortcutBinding>,
    pub push_to_talk: bool,
    pub audio_feedback: bool,
    #[serde(default = "default_audio_feedback_volume")]
    pub audio_feedback_volume: f32,

    /// Software gain applied to captured microphone samples, as a linear
    /// multiplier. Lets a user with a quiet microphone raise their level from
    /// inside Trazo instead of the OS sound panel.
    ///
    /// This is a convenience control, NOT a fix for the long-dictation
    /// truncation: gain raises speech and room noise by the same factor, so
    /// the VAD sees an identical signal. Measured 2026-07-26 — see
    /// `audio_toolkit::input_gain`.
    #[serde(default = "default_microphone_gain")]
    pub microphone_gain: f32,
    #[serde(default = "default_sound_theme")]
    pub sound_theme: SoundTheme,
    #[serde(default = "default_start_hidden")]
    pub start_hidden: bool,
    #[serde(default = "default_autostart_enabled")]
    pub autostart_enabled: bool,
    #[serde(default = "default_update_checks_enabled")]
    pub update_checks_enabled: bool,
    #[serde(default = "default_show_whats_new_on_update")]
    pub show_whats_new_on_update: bool,
    /// The app version whose What's New the user has already seen. Fresh installs
    /// default to the current version (nothing is "new" to them). Existing users
    /// upgrading from before this key existed are blanked by the migration so they
    /// see the current release's notes — see `apply_settings_migrations`.
    #[serde(default = "default_whats_new_last_seen_version")]
    pub whats_new_last_seen_version: String,
    #[serde(default = "default_model")]
    pub selected_model: String,
    #[serde(default)]
    pub onboarding_completed: bool,
    #[serde(default = "default_always_on_microphone")]
    pub always_on_microphone: bool,
    #[serde(default)]
    pub selected_microphone: Option<String>,
    #[serde(default)]
    pub clamshell_microphone: Option<String>,
    #[serde(default)]
    pub selected_output_device: Option<String>,
    #[serde(default = "default_translate_to_english")]
    pub translate_to_english: bool,
    #[serde(default = "default_selected_language")]
    pub selected_language: String,
    #[serde(default = "default_overlay_position")]
    pub overlay_position: OverlayPosition,
    #[serde(default = "default_debug_mode")]
    pub debug_mode: bool,
    #[serde(default = "default_log_level")]
    pub log_level: LogLevel,
    #[serde(default)]
    pub custom_words: Vec<String>,
    /// User-authored expansion rules (abbreviation → full text) applied to the
    /// finished transcript. Empty by default, so existing stores need no
    /// migration.
    #[serde(default)]
    pub custom_replacements: Vec<CustomReplacement>,
    #[serde(default)]
    pub model_unload_timeout: ModelUnloadTimeout,
    #[serde(default = "default_word_correction_threshold")]
    pub word_correction_threshold: f64,
    #[serde(default = "default_history_limit")]
    pub history_limit: usize,
    #[serde(default = "default_recording_retention_period")]
    pub recording_retention_period: RecordingRetentionPeriod,
    #[serde(default)]
    pub paste_method: PasteMethod,
    #[serde(default)]
    pub clipboard_handling: ClipboardHandling,
    #[serde(default = "default_auto_submit")]
    pub auto_submit: bool,
    #[serde(default)]
    pub auto_submit_key: AutoSubmitKey,
    #[serde(default = "default_post_process_enabled")]
    pub post_process_enabled: bool,
    #[serde(default = "default_post_process_provider_id")]
    pub post_process_provider_id: String,
    #[serde(default = "default_post_process_providers")]
    pub post_process_providers: Vec<PostProcessProvider>,
    #[serde(default = "default_post_process_api_keys")]
    pub post_process_api_keys: SecretMap,
    #[serde(default = "default_post_process_models")]
    pub post_process_models: HashMap<String, String>,
    #[serde(default = "default_post_process_prompts")]
    pub post_process_prompts: Vec<LLMPrompt>,
    #[serde(default)]
    pub post_process_selected_prompt_id: Option<String>,
    /// Nombre con el que firma el formalizador. Vacío = sin firma.
    #[serde(default)]
    pub user_full_name: String,
    /// Tú o usted en los correos formalizados.
    #[serde(default)]
    pub formality_treatment: FormalityTreatment,
    /// Perfil que ejecuta el atajo de formalizar. Independiente de
    /// `post_process_selected_prompt_id` para no obligar a pasar por Ajustes.
    #[serde(default)]
    pub formalize_prompt_id: Option<String>,
    #[serde(default)]
    pub mute_while_recording: bool,
    /// System output volume to hold while recording: `None` leaves the volume
    /// alone, `Some(0.0)` mutes, anything above ducks to that level (0.0-1.0).
    /// Levels above zero are only honored on Windows for now.
    #[serde(default)]
    pub recording_volume: Option<f32>,
    #[serde(default)]
    pub append_trailing_space: bool,
    #[serde(default = "default_app_language")]
    pub app_language: String,
    #[serde(default)]
    pub experimental_enabled: bool,
    #[serde(default)]
    pub lazy_stream_close: bool,
    #[serde(default)]
    pub keyboard_implementation: KeyboardImplementation,
    #[serde(default = "default_show_tray_icon")]
    pub show_tray_icon: bool,
    #[serde(default = "default_paste_delay_ms")]
    pub paste_delay_ms: u64,
    #[serde(default = "default_typing_tool")]
    pub typing_tool: TypingTool,
    pub external_script_path: Option<String>,
    #[serde(default)]
    pub custom_filler_words: Option<Vec<String>>,
    #[serde(default)]
    pub transcribe_accelerator: TranscribeAcceleratorSetting,
    #[serde(default)]
    pub ort_accelerator: OrtAcceleratorSetting,
    #[serde(default = "default_transcribe_gpu_device")]
    pub transcribe_gpu_device: i32,
    #[serde(default)]
    pub extra_recording_buffer_ms: u64,
    #[serde(default = "default_vad_enabled")]
    pub vad_enabled: bool,
    /// Which recording overlay to show: None / Minimal / Live. Streaming mode is
    /// not gated on this — that follows model capability. Migrated from the old
    /// `overlay_position` (position `none` → style `None`).
    #[serde(default = "default_overlay_style")]
    pub overlay_style: OverlayStyle,
}

fn default_model() -> String {
    "".to_string()
}

const CURRENT_SETTINGS_SCHEMA_VERSION: u32 = 8;

fn default_settings_schema_version() -> u32 {
    CURRENT_SETTINGS_SCHEMA_VERSION
}

fn default_always_on_microphone() -> bool {
    false
}

fn default_translate_to_english() -> bool {
    false
}

fn default_start_hidden() -> bool {
    false
}

fn default_autostart_enabled() -> bool {
    false
}

fn default_update_checks_enabled() -> bool {
    true
}

fn default_show_whats_new_on_update() -> bool {
    true
}

fn default_whats_new_last_seen_version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}

fn default_selected_language() -> String {
    "auto".to_string()
}

fn default_overlay_position() -> OverlayPosition {
    // Position only matters when the overlay is shown; whether it shows at all is
    // `overlay_style` (Linux defaults that to None). So a single default suffices.
    OverlayPosition::Bottom
}

fn default_overlay_style() -> OverlayStyle {
    // Linux hides the overlay by default; other platforms show the live overlay.
    // Position is independent and only selects top vs. bottom placement.
    #[cfg(target_os = "linux")]
    return OverlayStyle::None;
    #[cfg(not(target_os = "linux"))]
    return OverlayStyle::Live;
}

/// Off by default since 2026-07-28.
///
/// On a multi-channel Realtek capture device the VAD dropped almost the whole
/// dictation *during capture*: a 13 s recording landed on disk as 1.05-2.16 s.
/// The trade is lopsided — with the VAD off a dictation carries some extra
/// silence, which the model handles; with it on, this device loses ~90% of what
/// was said, silently.
///
/// Revisit once `speech_gate_truncates_multichannel_capture` is understood; the
/// setting is still exposed, so anyone it works for can switch it back on.
fn default_vad_enabled() -> bool {
    false
}

fn default_debug_mode() -> bool {
    false
}

fn default_log_level() -> LogLevel {
    LogLevel::Debug
}

fn default_word_correction_threshold() -> f64 {
    0.18
}

fn default_paste_delay_ms() -> u64 {
    60
}

fn default_auto_submit() -> bool {
    false
}

/// The history limit shipped before 2026-07-26. Kept so the schema v5
/// migration can tell an untouched default from a number the user chose.
pub const LEGACY_HISTORY_LIMIT: usize = 5;

/// Kept deliberately generous: recordings are small (a 30 s dictation is well
/// under 1 MB at 16 kHz mono), and a limit of five silently deleted three
/// separate evaluation corpora in July 2026 — once while the files were being
/// copied out. Twenty comfortably holds a 10-15 recording test batch for a
/// few tens of megabytes.
fn default_history_limit() -> usize {
    20
}

fn default_recording_retention_period() -> RecordingRetentionPeriod {
    RecordingRetentionPeriod::PreserveLimit
}

/// Unity: a fresh install must sound exactly like it did before this setting
/// existed.
fn default_microphone_gain() -> f32 {
    crate::audio_toolkit::UNITY_GAIN
}

/// Keep a stored gain inside the range the slider offers. The UI cannot
/// produce anything else, but a hand-edited store can, and a zero would be
/// indistinguishable from a dead microphone.
fn sanitize_microphone_gain(gain: f32) -> f32 {
    if !gain.is_finite() {
        return default_microphone_gain();
    }
    gain.clamp(
        crate::audio_toolkit::MIN_GAIN,
        crate::audio_toolkit::MAX_GAIN,
    )
}

fn default_audio_feedback_volume() -> f32 {
    1.0
}

fn default_sound_theme() -> SoundTheme {
    SoundTheme::Marimba
}

fn default_post_process_enabled() -> bool {
    false
}

fn default_app_language() -> String {
    tauri_plugin_os::locale()
        .map(|l| l.replace('_', "-"))
        .unwrap_or_else(|| "en".to_string())
}

fn default_show_tray_icon() -> bool {
    true
}

fn default_post_process_provider_id() -> String {
    "openai".to_string()
}

fn default_post_process_providers() -> Vec<PostProcessProvider> {
    let mut providers = vec![
        PostProcessProvider {
            id: "openai".to_string(),
            label: "OpenAI".to_string(),
            base_url: "https://api.openai.com/v1".to_string(),
            allow_base_url_edit: false,
            models_endpoint: Some("/models".to_string()),
            supports_structured_output: true,
        },
        PostProcessProvider {
            id: "zai".to_string(),
            label: "Z.AI".to_string(),
            base_url: "https://api.z.ai/api/paas/v4".to_string(),
            allow_base_url_edit: false,
            models_endpoint: Some("/models".to_string()),
            supports_structured_output: true,
        },
        PostProcessProvider {
            id: "openrouter".to_string(),
            label: "OpenRouter".to_string(),
            base_url: "https://openrouter.ai/api/v1".to_string(),
            allow_base_url_edit: false,
            models_endpoint: Some("/models".to_string()),
            supports_structured_output: true,
        },
        PostProcessProvider {
            id: "anthropic".to_string(),
            label: "Anthropic".to_string(),
            base_url: "https://api.anthropic.com/v1".to_string(),
            allow_base_url_edit: false,
            models_endpoint: Some("/models".to_string()),
            supports_structured_output: false,
        },
        PostProcessProvider {
            id: "groq".to_string(),
            label: "Groq".to_string(),
            base_url: "https://api.groq.com/openai/v1".to_string(),
            allow_base_url_edit: false,
            models_endpoint: Some("/models".to_string()),
            supports_structured_output: false,
        },
        PostProcessProvider {
            id: "cerebras".to_string(),
            label: "Cerebras".to_string(),
            base_url: "https://api.cerebras.ai/v1".to_string(),
            allow_base_url_edit: false,
            models_endpoint: Some("/models".to_string()),
            supports_structured_output: true,
        },
    ];

    // Note: We always include Apple Intelligence on macOS ARM64 without checking availability
    // at startup. The availability check is deferred to when the user actually tries to use it
    // (in actions.rs). This prevents crashes on macOS 26.x beta where accessing
    // SystemLanguageModel.default during early app initialization causes SIGABRT.
    #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
    {
        providers.push(PostProcessProvider {
            id: APPLE_INTELLIGENCE_PROVIDER_ID.to_string(),
            label: "Apple Intelligence".to_string(),
            base_url: "apple-intelligence://local".to_string(),
            allow_base_url_edit: false,
            models_endpoint: None,
            supports_structured_output: true,
        });
    }

    // AWS Bedrock via Mantle (OpenAI-compatible endpoint)
    providers.push(PostProcessProvider {
        id: "bedrock_mantle".to_string(),
        label: "AWS Bedrock (Mantle)".to_string(),
        base_url: "https://bedrock-mantle.us-east-1.api.aws/v1".to_string(),
        allow_base_url_edit: false,
        models_endpoint: Some("/models".to_string()),
        supports_structured_output: true,
    });

    // Custom provider always comes last
    providers.push(PostProcessProvider {
        id: "custom".to_string(),
        label: "Custom".to_string(),
        base_url: "http://localhost:11434/v1".to_string(),
        allow_base_url_edit: true,
        models_endpoint: Some("/models".to_string()),
        supports_structured_output: false,
    });

    providers
}

fn default_post_process_api_keys() -> SecretMap {
    let mut map = HashMap::new();
    for provider in default_post_process_providers() {
        map.insert(provider.id, String::new());
    }
    SecretMap(map)
}

fn default_model_for_provider(provider_id: &str) -> String {
    if provider_id == APPLE_INTELLIGENCE_PROVIDER_ID {
        return APPLE_INTELLIGENCE_DEFAULT_MODEL_ID.to_string();
    }
    String::new()
}

fn default_post_process_models() -> HashMap<String, String> {
    let mut map = HashMap::new();
    for provider in default_post_process_providers() {
        map.insert(
            provider.id.clone(),
            default_model_for_provider(&provider.id),
        );
    }
    map
}

/// Shared core of the seeded Spanish dictation profiles: spoken
/// self-corrections, the AI/automation community tech glossary, and cleanup
/// rules. Each profile appends its own output-format block on top.
const SPANISH_PROFILE_CORE: &str = r#"Eres el post-procesador de un dictado por voz en español de una comunidad de IA y automatización. Recibirás la transcripción cruda de un dictado. Devuelve ÚNICAMENTE el texto final, sin comentarios, sin comillas envolventes y sin explicaciones.

REGLAS (aplícalas en este orden):

1. AUTOCORRECCIONES HABLADAS: si el hablante se corrige a sí mismo, conserva SOLO la versión final y elimina la parte descartada y el marcador de corrección.
   - "el martes... no, mejor el jueves" → "el jueves"
   - "envíaselo a Juan, digo, a Pedro" → "envíaselo a Pedro"
   - "tres reintentos, espera, mejor cinco reintentos" → "cinco reintentos"
   Marcadores típicos: "no, mejor", "digo", "perdón", "quise decir", "bueno no", "espera", "mejor dicho", "borra eso".

2. GLOSARIO TÉCNICO: estos términos NUNCA se traducen; si la transcripción los deformó, corrige a la forma canónica exacta:
   commit, pull request ("pul reques" → "pull request"), merge, deploy, rollback, webhook ("güebjuc", "web juc" → "webhook"), endpoint, workflow, prompt, n8n ("ene ocho ene", "n eight n" → "n8n"), API, token, backend, frontend, repo, branch, pipeline, script, plugin, dashboard, LLM, embedding, fine-tuning.
   El resto del texto va en español natural.

3. LIMPIEZA: corrige puntuación, mayúsculas y ortografía; elimina muletillas ("eh", "este", "o sea" cuando son relleno); convierte números hablados a cifras (veinticinco → 25).

4. Conserva el significado exacto y el registro del hablante. Mantén el idioma del dictado salvo que el FORMATO DE SALIDA indique otra cosa."#;

/// The profile selected for fresh installs and for stores that had no
/// selection (a `None` selection makes the post-process hotkey silently no-op).
pub const DEFAULT_SELECTED_PROMPT_ID: &str = "default_es_casual";

/// Id del perfil de correo sembrado, al que apunta `formalize_prompt_id`.
pub const DEFAULT_EMAIL_PROMPT_ID: &str = "default_es_email";

fn spanish_profile_prompt(format_block: &str) -> String {
    // Ends with the `${output}` placeholder so the legacy (non-structured)
    // LLM path still receives the transcript; the structured path strips it.
    format!("{SPANISH_PROFILE_CORE}\n\n{format_block}\n\nTranscripción:\n${{output}}")
}

fn default_post_process_prompts() -> Vec<LLMPrompt> {
    vec![
        LLMPrompt {
            id: "default_improve_transcriptions".to_string(),
            name: "Improve Transcriptions".to_string(),
            prompt: "Clean this transcript:\n1. Fix spelling, capitalization, and punctuation errors\n2. Convert number words to digits (twenty-five → 25, ten percent → 10%, five dollars → $5)\n3. Replace spoken punctuation with symbols (period → ., comma → ,, question mark → ?)\n4. Remove filler words (um, uh, like as filler)\n5. Keep the language in the original version (if it was french, keep it in french for example)\n\nPreserve exact meaning and word order. Do not paraphrase or reorder content.\n\nReturn only the cleaned transcript.\n\nTranscript:\n${output}".to_string(),
        },
        LLMPrompt {
            id: DEFAULT_SELECTED_PROMPT_ID.to_string(),
            name: "Mensaje casual (ES)".to_string(),
            prompt: spanish_profile_prompt(
                "FORMATO DE SALIDA: mensaje de chat (WhatsApp/Discord/Slack): tono cercano, frases cortas, sin añadir saludos ni despedidas que no estén en el dictado, en un solo párrafo salvo que el dictado enumere cosas. No parafrasees ni reordenes el contenido.",
            ),
        },
        LLMPrompt {
            id: "default_es_commit".to_string(),
            name: "Commit convencional (ES→EN)".to_string(),
            prompt: spanish_profile_prompt(
                "FORMATO DE SALIDA: un conventional commit EN INGLÉS. Primera línea: \"tipo(scope opcional): descripción imperativa en minúsculas\", de máximo 72 caracteres; tipos permitidos: feat, fix, docs, refactor, chore, test, perf. Si el dictado aporta contexto adicional, añádelo como cuerpo tras una línea en blanco, explicando el porqué del cambio. Los términos del glosario se conservan tal cual. Aquí SÍ debes reestructurar el dictado al formato del commit y traducir la descripción al inglés.",
            ),
        },
        LLMPrompt {
            id: "default_es_community".to_string(),
            name: "Post comunidad (ES)".to_string(),
            prompt: spanish_profile_prompt(
                "FORMATO DE SALIDA: un post estructurado para la comunidad: primera línea como título en **negrita**, después 1-3 párrafos cortos, con viñetas si el dictado enumera pasos o ideas, y cierre con pregunta o llamado a la acción SOLO si el dictado lo contiene. Aquí SÍ puedes reorganizar el contenido para darle estructura.",
            ),
        },
        LLMPrompt {
            id: DEFAULT_EMAIL_PROMPT_ID.to_string(),
            name: "Correo formal (ES)".to_string(),
            prompt: r#"Eres el post-procesador de un dictado por voz en español. Recibirás la transcripción cruda de un dictado y debes convertirla en un CORREO listo para enviar. Devuelve ÚNICAMENTE el texto final, sin comentarios, sin comillas envolventes y sin explicaciones.

TRATAMIENTO: dirígete al destinatario de ${tratamiento}. Conjuga TODOS los verbos y pronombres en consecuencia, sin mezclar los dos tratamientos.

ESTRUCTURA, en este orden:

1. SALUDO: empieza exactamente por "${saludo}". Si el dictado dice a quién va dirigido el mensaje, añade su nombre: "${saludo}, María:". Si no menciona destinatario, deja "${saludo}:" a secas. NUNCA inventes un nombre, y no confundas a quién va dirigido con quién se menciona de pasada.
2. CUERPO: reescribe el dictado en 1-3 párrafos cortos, en registro profesional pero natural. Corrige puntuación, mayúsculas y ortografía; elimina muletillas; convierte números hablados a cifras (veinticinco → 25). Si el hablante se corrige a sí mismo, conserva SOLO la versión final ("el martes... no, mejor el jueves" → "el jueves").
3. DESPEDIDA + FIRMA: el valor exacto del nombre del firmante es lo que hay entre estas comillas angulares: «${nombre_usuario}». Mira con atención qué hay entre «» antes de decidir:
   - Si entre «» hay al menos un carácter (ejemplo: «Charly»): NO ESTÁ VACÍO. Escribe una despedida breve terminada en COMA ("Un saludo," o "Quedo atento,"), y en la línea siguiente el nombre solo, sin nada más debajo.
   - Si entre «» no hay ningún carácter, es decir ves «» pegado y vacío: SÍ ESTÁ VACÍO, no hay firma. Escribe la misma despedida pero terminada en PUNTO, nunca en coma ("Un saludo." o "Quedo atento."), y esa despedida es la ÚLTIMA línea del correo: no la sigas de ninguna línea de firma ni de ninguna línea en blanco.
   - MAL (prohibido cuando «» está vacío): "Quedo atento," seguido de nada o de una línea vacía. BIEN: "Quedo atento." y ahí termina el correo. Antes de dar tu respuesta por buena, mira la última línea: si no escribiste un nombre de persona en ella, esa línea no puede terminar en coma.

REGLAS:
- No añadas información que no esté en el dictado. No inventes asuntos, fechas ni compromisos.
- Conserva el significado exacto. Sí puedes reorganizar el contenido para darle forma de correo.
- Los términos técnicos en inglés se mantienen tal cual: commit, pull request, merge, deploy, rollback, webhook, endpoint, workflow, prompt, API, token, backend, frontend, repo, branch, pipeline.

Dos ejemplos completos, uno con firma y otro sin firma — usa el que de verdad corresponda al
valor que viste arriba entre «», NO copies siempre el mismo:

EJEMPLO CON FIRMA:

Buenas tardes, Ana:

Te confirmo que el despliegue quedó completado sin problemas.

Quedo atento,
Charly

EJEMPLO SIN FIRMA:

Buenas tardes, Ana:

Te confirmo que el despliegue quedó completado sin problemas.

Quedo atento.

RECUERDA antes de responder: el valor real de la firma en ESTE dictado es «${nombre_usuario}».
Comprueba si eso está vacío o no, y sigue el ejemplo (CON FIRMA o SIN FIRMA) que de verdad le
corresponda.

Transcripción:
${output}"#
                .to_string(),
        },
    ]
}

fn default_transcribe_gpu_device() -> i32 {
    -1 // auto
}

fn default_typing_tool() -> TypingTool {
    TypingTool::Auto
}

fn ensure_post_process_defaults(settings: &mut AppSettings) -> bool {
    let mut changed = false;
    for provider in default_post_process_providers() {
        // Use match to do a single lookup - either sync existing or add new
        match settings
            .post_process_providers
            .iter_mut()
            .find(|p| p.id == provider.id)
        {
            Some(existing) => {
                // Sync supports_structured_output field for existing providers (migration)
                if existing.supports_structured_output != provider.supports_structured_output {
                    debug!(
                        "Updating supports_structured_output for provider '{}' from {} to {}",
                        provider.id,
                        existing.supports_structured_output,
                        provider.supports_structured_output
                    );
                    existing.supports_structured_output = provider.supports_structured_output;
                    changed = true;
                }
            }
            None => {
                // Provider doesn't exist, add it
                settings.post_process_providers.push(provider.clone());
                changed = true;
            }
        }

        if !settings.post_process_api_keys.contains_key(&provider.id) {
            settings
                .post_process_api_keys
                .insert(provider.id.clone(), String::new());
            changed = true;
        }

        let default_model = default_model_for_provider(&provider.id);
        match settings.post_process_models.get_mut(&provider.id) {
            Some(existing) => {
                if existing.is_empty() && !default_model.is_empty() {
                    *existing = default_model.clone();
                    changed = true;
                }
            }
            None => {
                settings
                    .post_process_models
                    .insert(provider.id.clone(), default_model);
                changed = true;
            }
        }
    }

    changed
}

pub const SETTINGS_STORE_PATH: &str = "settings_store.json";

pub fn get_default_settings() -> AppSettings {
    #[cfg(target_os = "windows")]
    let default_shortcut = "ctrl+space";
    #[cfg(target_os = "macos")]
    let default_shortcut = "option+space";
    #[cfg(target_os = "linux")]
    let default_shortcut = "ctrl+space";
    #[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
    let default_shortcut = "alt+space";

    let mut bindings = HashMap::new();
    bindings.insert(
        "transcribe".to_string(),
        ShortcutBinding {
            id: "transcribe".to_string(),
            name: "Transcribe".to_string(),
            description: "Converts your speech into text.".to_string(),
            default_binding: default_shortcut.to_string(),
            current_binding: default_shortcut.to_string(),
        },
    );
    #[cfg(target_os = "windows")]
    let default_post_process_shortcut = "ctrl+shift+space";
    #[cfg(target_os = "macos")]
    let default_post_process_shortcut = "option+shift+space";
    #[cfg(target_os = "linux")]
    let default_post_process_shortcut = "ctrl+shift+space";
    #[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
    let default_post_process_shortcut = "alt+shift+space";

    bindings.insert(
        "transcribe_with_post_process".to_string(),
        ShortcutBinding {
            id: "transcribe_with_post_process".to_string(),
            name: "Transcribe with Post-Processing".to_string(),
            description: "Converts your speech into text and applies AI post-processing."
                .to_string(),
            default_binding: default_post_process_shortcut.to_string(),
            current_binding: default_post_process_shortcut.to_string(),
        },
    );

    // Una sola tecla, nunca un acorde ni un modificador desnudo: un modificador
    // (ctrl_right en Windows/Linux, cmd_right en macOS) se traga la propia
    // PULSACION cuando el estado resultante coincide con el hotkey — Ctrl+C
    // arranca una grabación y la app enfocada recibe una "c" literal, y en
    // macOS es peor porque Command es el modificador de casi todos los atajos
    // del sistema. Bajo la implementación Tauri (la de por defecto en Linux)
    // "ctrl_right" además falla a la hora de parsear tras pasar la validación,
    // dejando el atajo mudo en silencio. F9 es una sola tecla física en todos
    // los teclados, no es modificador (nada que tragarse) y el parser de Tauri
    // sí la reconoce: mismo default en las tres plataformas.
    let default_formalize_shortcut = "f9";

    bindings.insert(
        "transcribe_and_formalize".to_string(),
        ShortcutBinding {
            id: "transcribe_and_formalize".to_string(),
            name: "Transcribe and Formalize".to_string(),
            description: "Dictates and rewrites it as a ready-to-send email.".to_string(),
            default_binding: default_formalize_shortcut.to_string(),
            current_binding: default_formalize_shortcut.to_string(),
        },
    );
    bindings.insert(
        "cancel".to_string(),
        ShortcutBinding {
            id: "cancel".to_string(),
            name: "Cancel".to_string(),
            description: "Cancels the current recording.".to_string(),
            default_binding: "escape".to_string(),
            current_binding: "escape".to_string(),
        },
    );

    AppSettings {
        settings_schema_version: default_settings_schema_version(),
        bindings,
        push_to_talk: true,
        audio_feedback: false,
        audio_feedback_volume: default_audio_feedback_volume(),
        microphone_gain: default_microphone_gain(),
        sound_theme: default_sound_theme(),
        start_hidden: default_start_hidden(),
        autostart_enabled: default_autostart_enabled(),
        update_checks_enabled: default_update_checks_enabled(),
        show_whats_new_on_update: default_show_whats_new_on_update(),
        whats_new_last_seen_version: default_whats_new_last_seen_version(),
        selected_model: "".to_string(),
        onboarding_completed: false,
        always_on_microphone: false,
        selected_microphone: None,
        clamshell_microphone: None,
        selected_output_device: None,
        translate_to_english: false,
        selected_language: "auto".to_string(),
        overlay_position: default_overlay_position(),
        debug_mode: false,
        log_level: default_log_level(),
        custom_words: Vec::new(),
        custom_replacements: Vec::new(),
        model_unload_timeout: ModelUnloadTimeout::default(),
        word_correction_threshold: default_word_correction_threshold(),
        history_limit: default_history_limit(),
        recording_retention_period: default_recording_retention_period(),
        paste_method: PasteMethod::default(),
        clipboard_handling: ClipboardHandling::default(),
        auto_submit: default_auto_submit(),
        auto_submit_key: AutoSubmitKey::default(),
        post_process_enabled: default_post_process_enabled(),
        post_process_provider_id: default_post_process_provider_id(),
        post_process_providers: default_post_process_providers(),
        post_process_api_keys: default_post_process_api_keys(),
        post_process_models: default_post_process_models(),
        post_process_prompts: default_post_process_prompts(),
        post_process_selected_prompt_id: Some(DEFAULT_SELECTED_PROMPT_ID.to_string()),
        user_full_name: String::new(),
        formality_treatment: FormalityTreatment::Tu,
        formalize_prompt_id: Some(DEFAULT_EMAIL_PROMPT_ID.to_string()),
        mute_while_recording: false,
        recording_volume: None,
        append_trailing_space: false,
        app_language: default_app_language(),
        experimental_enabled: false,
        lazy_stream_close: false,
        keyboard_implementation: KeyboardImplementation::default(),
        show_tray_icon: default_show_tray_icon(),
        paste_delay_ms: default_paste_delay_ms(),
        typing_tool: default_typing_tool(),
        external_script_path: None,
        custom_filler_words: None,
        transcribe_accelerator: TranscribeAcceleratorSetting::default(),
        ort_accelerator: OrtAcceleratorSetting::default(),
        transcribe_gpu_device: default_transcribe_gpu_device(),
        extra_recording_buffer_ms: 0,
        vad_enabled: default_vad_enabled(),
        overlay_style: default_overlay_style(),
    }
}

impl AppSettings {
    pub fn active_post_process_provider(&self) -> Option<&PostProcessProvider> {
        self.post_process_providers
            .iter()
            .find(|provider| provider.id == self.post_process_provider_id)
    }

    pub fn post_process_provider(&self, provider_id: &str) -> Option<&PostProcessProvider> {
        self.post_process_providers
            .iter()
            .find(|provider| provider.id == provider_id)
    }

    pub fn post_process_provider_mut(
        &mut self,
        provider_id: &str,
    ) -> Option<&mut PostProcessProvider> {
        self.post_process_providers
            .iter_mut()
            .find(|provider| provider.id == provider_id)
    }
}

pub fn load_or_create_app_settings(app: &AppHandle) -> AppSettings {
    // Initialize store
    let store = app
        .store(crate::portable::store_path(SETTINGS_STORE_PATH))
        .expect("Failed to initialize store");

    let mut settings = if let Some(settings_value) = store.get("settings") {
        // Parse the entire settings object
        match serde_json::from_value::<AppSettings>(settings_value.clone()) {
            Ok(mut settings) => {
                debug!("Found existing settings: {:?}", settings);
                let default_settings = get_default_settings();
                let mut updated = apply_settings_migrations(&mut settings, &settings_value);

                // Merge default bindings into existing settings
                for (key, value) in default_settings.bindings {
                    if let std::collections::hash_map::Entry::Vacant(entry) =
                        settings.bindings.entry(key)
                    {
                        debug!("Adding missing binding: {}", entry.key());
                        entry.insert(value);
                        updated = true;
                    }
                }

                if updated {
                    debug!("Settings updated with defaults/migrations");
                    store.set("settings", serde_json::to_value(&settings).unwrap());
                }

                settings
            }
            Err(e) => {
                warn!("Failed to parse settings: {}", e);
                // Fall back to default settings if parsing fails
                let default_settings = get_default_settings();
                store.set("settings", serde_json::to_value(&default_settings).unwrap());
                default_settings
            }
        }
    } else {
        let default_settings = get_default_settings();
        store.set("settings", serde_json::to_value(&default_settings).unwrap());
        default_settings
    };

    if ensure_post_process_defaults(&mut settings) {
        store.set("settings", serde_json::to_value(&settings).unwrap());
    }

    settings
}

pub fn get_settings(app: &AppHandle) -> AppSettings {
    let store = app
        .store(crate::portable::store_path(SETTINGS_STORE_PATH))
        .expect("Failed to initialize store");

    // Settings reads also persist one-time migrations. Migration helpers are
    // idempotent, so this converges after the first read of an older store.
    let mut settings = if let Some(settings_value) = store.get("settings") {
        match serde_json::from_value::<AppSettings>(settings_value.clone()) {
            Ok(mut settings) => {
                if apply_settings_migrations(&mut settings, &settings_value) {
                    store.set("settings", serde_json::to_value(&settings).unwrap());
                }
                settings
            }
            Err(_) => {
                let default_settings = get_default_settings();
                store.set("settings", serde_json::to_value(&default_settings).unwrap());
                default_settings
            }
        }
    } else {
        let default_settings = get_default_settings();
        store.set("settings", serde_json::to_value(&default_settings).unwrap());
        default_settings
    };

    if ensure_post_process_defaults(&mut settings) {
        store.set("settings", serde_json::to_value(&settings).unwrap());
    }

    settings
}

fn apply_settings_migrations(
    settings: &mut AppSettings,
    settings_value: &serde_json::Value,
) -> bool {
    let mut updated = false;

    // One-time onboarding migration: users with an explicit selected model have
    // already made it through model selection. Users who merely have compatible
    // files on disk should still see onboarding.
    if settings_value.get("onboarding_completed").is_none() {
        settings.onboarding_completed = !settings.selected_model.is_empty();
        updated = true;
    }

    // One-time What's New migration: migrations only run on an existing store
    // (fresh installs stamp the current version via get_default_settings). A
    // missing key here means a user upgrading from before it existed — blank it
    // so they see the current release's What's New, mirroring the onboarding
    // migration's explicit first-run-vs-upgrade decision.
    if settings_value.get("whats_new_last_seen_version").is_none() {
        settings.whats_new_last_seen_version = String::new();
        updated = true;
    }

    let stored_schema_version = settings_value
        .get("settings_schema_version")
        .and_then(|v| v.as_u64())
        .unwrap_or(0);
    if stored_schema_version < 1 {
        // `transcribe_gpu_device` used to be a UI ordinal; it is now a
        // transcribe.cpp registry index. A positive legacy value can point at a
        // different GPU after CPU/accelerator/backend devices are included in
        // the registry, so reset ambiguous explicit selections to Auto once.
        if settings.transcribe_gpu_device > 0 {
            settings.transcribe_accelerator = TranscribeAcceleratorSetting::Auto;
            settings.transcribe_gpu_device = default_transcribe_gpu_device();
        }
        settings.settings_schema_version = CURRENT_SETTINGS_SCHEMA_VERSION;
        updated = true;
    }

    if stored_schema_version < 2 {
        // Seed the Spanish dictation profiles introduced in schema v2 without
        // clobbering user-edited prompts: append only ids that are absent, and
        // only pick a selection when there was none (a `None` selection makes
        // the post-process hotkey silently no-op, which reads as "broken").
        for prompt in default_post_process_prompts() {
            if !settings
                .post_process_prompts
                .iter()
                .any(|p| p.id == prompt.id)
            {
                settings.post_process_prompts.push(prompt);
            }
        }
        if settings.post_process_selected_prompt_id.is_none() {
            settings.post_process_selected_prompt_id = Some(DEFAULT_SELECTED_PROMPT_ID.to_string());
        }
        settings.settings_schema_version = CURRENT_SETTINGS_SCHEMA_VERSION;
        updated = true;
    }

    if stored_schema_version < 3 {
        // One-time product-default flip (its own schema step so it also reaches
        // stores an intermediate build already bumped to v2): keep the
        // transcript on the clipboard after pasting so a silent paste failure
        // never loses a dictation. Users who prefer DontModify can re-select
        // it once in settings.
        settings.clipboard_handling = ClipboardHandling::CopyToClipboard;
        settings.settings_schema_version = CURRENT_SETTINGS_SCHEMA_VERSION;
        updated = true;
    }

    if stored_schema_version < 4 {
        // Fold the binary mute into the recording-volume model: mute becomes
        // "duck to 0". The legacy flag is turned off so both paths never apply
        // at once, and an explicit duck level a user already set wins.
        if settings.mute_while_recording {
            if settings.recording_volume.is_none() {
                settings.recording_volume = Some(0.0);
            }
            settings.mute_while_recording = false;
        }
        settings.settings_schema_version = CURRENT_SETTINGS_SCHEMA_VERSION;
        updated = true;
    }

    if stored_schema_version < 5 {
        // Raise the history limit for anyone still on the old default of five.
        // That default deleted recordings faster than they could be reviewed
        // (three evaluation corpora lost in July 2026), and with
        // `RecordingRetentionPeriod::PreserveLimit` it takes the WAV files with
        // it, so the audio is gone for good.
        //
        // Only the exact legacy value is touched: any other number is a choice
        // the user made in settings and must survive. Someone who deliberately
        // picked five is moved too — unavoidable, since the store keeps no
        // record of who chose it, and it is one click to set back.
        if settings.history_limit == LEGACY_HISTORY_LIMIT {
            settings.history_limit = default_history_limit();
        }
        settings.settings_schema_version = CURRENT_SETTINGS_SCHEMA_VERSION;
        updated = true;
    }

    if stored_schema_version < 6 {
        // Turn the VAD off on stores that predate 2026-07-28. On a
        // multi-channel Realtek capture device it dropped nearly the whole
        // dictation while recording — 13 s of speech reached disk as 1.05-2.16 s
        // — and the loss is silent: the user sees a normal recording and gets a
        // one-line transcript.
        //
        // Unlike the history-limit migration this cannot tell a deliberate
        // choice from the old default, because the old default WAS `true`. That
        // is accepted: the downside of turning it off for someone it worked for
        // is some extra silence in the audio, which the model handles, against
        // losing ~90% of every dictation for someone it did not. The setting
        // stays in the UI, so re-enabling is one click and survives this
        // migration (it only runs once).
        if settings.vad_enabled {
            settings.vad_enabled = false;
        }
        settings.settings_schema_version = CURRENT_SETTINGS_SCHEMA_VERSION;
        updated = true;
    }

    if stored_schema_version < 7 {
        // Siembra SOLO el perfil de correo, que es lo nuevo de este schema (a
        // diferencia de la v2, que sembraba los tres perfiles ES a la vez y por
        // eso recorria default_post_process_prompts() completo). Recorrer la
        // lista entera aqui resucitaria cualquier perfil que el usuario haya
        // borrado deliberadamente en v6 (p. ej. default_es_commit). No pisa un
        // prompt que el usuario ya haya editado. La selección global NO se
        // toca: el atajo de formalizar tiene su propio `formalize_prompt_id`.
        if !settings
            .post_process_prompts
            .iter()
            .any(|p| p.id == DEFAULT_EMAIL_PROMPT_ID)
        {
            if let Some(email_prompt) = default_post_process_prompts()
                .into_iter()
                .find(|p| p.id == DEFAULT_EMAIL_PROMPT_ID)
            {
                settings.post_process_prompts.push(email_prompt);
            }
        }
        if settings.formalize_prompt_id.is_none() {
            settings.formalize_prompt_id = Some(DEFAULT_EMAIL_PROMPT_ID.to_string());
        }
        settings.settings_schema_version = CURRENT_SETTINGS_SCHEMA_VERSION;
        updated = true;
    }

    if stored_schema_version < 8 {
        // Repara los stores que la interfaz volteó a `dont_modify` sin que nadie
        // lo eligiera: el desplegable pintaba esa opción como seleccionada
        // mientras los ajustes no habían cargado y escribía al pulsar la que ya
        // aparecía marcada. La v3 no alcanza a estos stores porque ya están muy
        // por encima de su versión, así que sin este paso quedarían rotos para
        // siempre.
        //
        // Mismo criterio que la v6 con el VAD, y con el mismo precio: no hay
        // forma de distinguir a quien eligió `dont_modify` a propósito de quien
        // lo sufrió, porque el store no guarda quién lo escribió. Se acepta
        // porque los dos lados no cuestan igual: a quien lo quería le sobra un
        // dictado en el portapapeles y lo vuelve a poner con un clic, mientras
        // que al afectado se le pierde el dictado entero y de forma silenciosa,
        // sin manera de recuperarlo. Corre una sola vez, así que la elección
        // repetida sobrevive.
        settings.clipboard_handling = ClipboardHandling::CopyToClipboard;
        settings.settings_schema_version = CURRENT_SETTINGS_SCHEMA_VERSION;
        updated = true;
    }

    // One-time overlay migration (only while the new key is absent): the retired
    // overlay_position `none` meant "hide the overlay" → OverlayStyle::None; any
    // other position had it visible → Live. The position enum no longer has a
    // `none` variant (legacy "none" deserializes to Bottom via a serde alias), so
    // read the raw stored string to recover the old intent.
    if settings_value.get("overlay_style").is_none() {
        let was_hidden = settings_value
            .get("overlay_position")
            .and_then(|v| v.as_str())
            == Some("none");
        settings.overlay_style = if was_hidden {
            OverlayStyle::None
        } else {
            OverlayStyle::Live
        };
        updated = true;
    }

    updated
}

pub fn write_settings(app: &AppHandle, settings: AppSettings) {
    let store = app
        .store(crate::portable::store_path(SETTINGS_STORE_PATH))
        .expect("Failed to initialize store");

    store.set("settings", serde_json::to_value(&settings).unwrap());

    restrict_settings_store_permissions(app);
}

/// VEKTRUN: 0600 sobre `settings_store.json`.
///
/// Comprobado en una instalacion real el 7-ago-2026: el fichero quedaba 0644, y
/// dentro va `post_process_api_keys` EN CLARO — las claves de OpenAI,
/// Anthropic, Groq y del proveedor propio. `SecretMap` solo redacta en `Debug`
/// (logs y trazas de panico); al serializar lleva `#[serde(transparent)]`, asi
/// que al disco van tal cual.
///
/// El primer intento de este endurecimiento solo cubrio `history.db` y
/// `recordings/`. Este fichero es MAS sensible que aquellos y se quedo fuera:
/// un dictado es un dictado, una clave abre la cuenta entera.
///
/// Se llama en cada escritura a proposito: el plugin de store puede recrear el
/// fichero, y un chmod sobre algo que ya esta a 0600 no cuesta nada. Si aun no
/// existe, no hay nada que hacer y se sale en silencio.
///
/// Esto NO sustituye a guardar las claves en el llavero del sistema, que es lo
/// correcto y sigue pendiente. Reduce la exposicion; no la elimina.
///
/// **Necesita el `AppHandle`, no vale `portable::store_path`.** El primer
/// intento uso `store_path`, que en modo NO portatil devuelve la ruta relativa
/// `"settings_store.json"` — el plugin de store la resuelve luego contra el
/// directorio de datos, pero nosotros no. El `exists()` daba falso y la funcion
/// salia en silencio: el fichero se quedo a 0644 y el codigo parecia correcto.
/// Se vio mirando los permisos reales de una instalacion, no leyendo el codigo.
pub fn restrict_settings_store_permissions(app: &AppHandle) {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;

        let dir = match crate::portable::app_data_dir(app) {
            Ok(dir) => dir,
            Err(e) => {
                log::warn!("No se pudo resolver el directorio de datos para el store: {e}");
                return;
            }
        };
        let path = dir.join(SETTINGS_STORE_PATH);

        if !path.exists() {
            // Solo es normal antes del primer guardado. Si sale despues, algo
            // no cuadra y queremos verlo: un salto silencioso aqui es lo que
            // dejo el fichero a 0644 la primera vez.
            log::debug!("El store aun no existe en {path:?}; nada que endurecer");
            return;
        }

        match std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600)) {
            Ok(()) => log::debug!("Permisos del store restringidos a 0600: {path:?}"),
            Err(e) => log::warn!("No se pudieron restringir los permisos de {path:?}: {e}"),
        }
    }

    #[cfg(not(unix))]
    {
        let _ = app;
    }
}

pub fn get_bindings(app: &AppHandle) -> HashMap<String, ShortcutBinding> {
    let settings = get_settings(app);

    settings.bindings
}

pub fn get_stored_binding(app: &AppHandle, id: &str) -> ShortcutBinding {
    let bindings = get_bindings(app);

    let binding = bindings.get(id).unwrap().clone();

    binding
}

/// The microphone gain to actually apply, clamped into the usable range.
///
/// VEKTRUN: valida la URL base del proveedor `custom` antes de guardarla.
///
/// El post-procesado envia la transcripcion ENTERA al endpoint configurado. Tal
/// como venia, el setter solo comprobaba que el proveedor fuera `custom` y
/// asignaba la cadena tal cual: `http://198.51.100.7:11434/v1` se aceptaba sin
/// un error y sin un aviso, y el dictado salia en claro por la red.
///
/// La regla es la mas estricta que no rompe el caso legitimo:
/// - `https://` a cualquier host — cifrado extremo a extremo, se acepta.
/// - `http://` SOLO a loopback — es el caso de Ollama en la propia maquina,
///   donde el trafico no toca la red.
/// - cualquier otra cosa se rechaza con el motivo escrito.
///
/// Un Ollama en OTRA maquina (p. ej. `msvr`) NO entra por `http://`: o va por
/// HTTPS, o el tunel (WireGuard) es quien cifra y entonces la URL sigue siendo
/// `http://` a una IP privada — caso que este validador rechaza a proposito.
/// Preferimos el falso positivo: que alguien tenga que pararse a pensar antes
/// de mandar dictados en claro.
///
/// Es pura para poder probarla sin red ni `AppSettings`.
pub fn validate_custom_base_url(base_url: &str) -> Result<(), String> {
    let trimmed = base_url.trim();

    if trimmed.is_empty() {
        return Err("La URL base no puede estar vacia".to_string());
    }

    let (scheme, rest) = match trimmed.split_once("://") {
        Some(parts) => parts,
        None => {
            return Err(format!(
                "La URL base debe empezar por https:// (o http:// si es local). Recibido: {trimmed}"
            ))
        }
    };

    match scheme.to_ascii_lowercase().as_str() {
        "https" => Ok(()),
        "http" => {
            // El host es lo que va antes de la primera '/', y sin credenciales
            // ni puerto.
            let authority = rest.split('/').next().unwrap_or("");
            let authority = authority.rsplit('@').next().unwrap_or(authority);

            let host = if let Some(end) = authority.strip_prefix('[') {
                // IPv6 literal: [::1]:11434
                end.split(']').next().unwrap_or("")
            } else {
                authority.split(':').next().unwrap_or("")
            };

            if is_loopback_host(host) {
                Ok(())
            } else {
                Err(format!(
                    "http:// solo se admite contra la propia maquina (localhost o 127.0.0.1). \
                     '{host}' es remoto: el dictado viajaria en claro. Usa https://."
                ))
            }
        }
        other => Err(format!(
            "Esquema '{other}' no admitido. Usa https:// (o http:// si es local)."
        )),
    }
}

/// Loopback segun el propio host, sin resolver DNS (resolver aqui abriria la
/// puerta a que un nombre que hoy apunta a 127.0.0.1 apunte manana a otro sitio).
///
/// La comparacion numerica va por `IpAddr::is_loopback`, NO por
/// `starts_with("127.")`: ese prefijo aceptaria `127.0.0.1.evil.com`, que es un
/// nombre de dominio corriente que empieza igual y resuelve donde su dueno
/// quiera. Parsear como IP lo rechaza porque no es una IP.
fn is_loopback_host(host: &str) -> bool {
    let host = host.trim().to_ascii_lowercase();

    if host == "localhost" {
        return true;
    }

    match host.parse::<std::net::IpAddr>() {
        Ok(ip) => ip.is_loopback(),
        // No es una IP: solo vale el literal "localhost", ya comprobado.
        // Cualquier otro nombre exigiria resolver DNS, y eso no se hace aqui.
        Err(_) => false,
    }
}

/// Goes through [`sanitize_microphone_gain`] rather than reading the field
/// directly, because the store is a plain JSON file a user can edit.
pub fn effective_microphone_gain(settings: &AppSettings) -> f32 {
    sanitize_microphone_gain(settings.microphone_gain)
}

pub fn get_history_limit(app: &AppHandle) -> usize {
    let settings = get_settings(app);
    settings.history_limit
}

pub fn get_recording_retention_period(app: &AppHandle) -> RecordingRetentionPeriod {
    let settings = get_settings(app);
    settings.recording_retention_period
}

#[cfg(test)]
mod vektrun_base_url_tests {
    use super::validate_custom_base_url as check;

    #[test]
    fn https_a_cualquier_host_se_acepta() {
        assert!(check("https://api.openai.com/v1").is_ok());
        assert!(check("https://llm.vektrun.com/v1").is_ok());
        assert!(check("https://198.51.100.7:8443/v1").is_ok());
    }

    #[test]
    fn http_a_loopback_se_acepta() {
        // El caso real: Ollama en la propia maquina. El trafico no toca la red.
        assert!(check("http://localhost:11434/v1").is_ok());
        assert!(check("http://127.0.0.1:11434/v1").is_ok());
        assert!(check("http://127.1.2.3:11434/v1").is_ok());
        assert!(check("http://[::1]:11434/v1").is_ok());
    }

    #[test]
    fn http_a_host_remoto_se_rechaza() {
        // Este es el agujero que se cierra: antes se guardaba sin decir nada y
        // el dictado entero salia en claro por la red.
        let err = check("http://198.51.100.7:11434/v1").unwrap_err();
        assert!(err.contains("198.51.100.7"), "got {err}");
        assert!(err.contains("claro"), "el motivo tiene que estar escrito: {err}");

        assert!(check("http://msvr:11434/v1").is_err());
        assert!(check("http://10.0.0.5:11434/v1").is_err());
    }

    #[test]
    fn un_host_que_solo_empieza_como_loopback_no_cuela() {
        // Este es el motivo de que la comprobacion numerica sea
        // `IpAddr::is_loopback` y no `starts_with("127.")`: los tres de abajo
        // empiezan igual que un loopback y son dominios de un tercero.
        assert!(
            check("http://127.0.0.1.evil.com/v1").is_err(),
            "un dominio que EMPIEZA por 127. no es loopback"
        );
        assert!(check("http://localhost.evil.com/v1").is_err());
        assert!(check("http://notlocalhost/v1").is_err());
    }

    #[test]
    fn credenciales_en_la_url_no_disfrazan_el_host() {
        // `http://127.0.0.1@evil.com/` tiene el host EVIL.COM, no loopback:
        // todo lo anterior al ultimo '@' es usuario:clave.
        assert!(
            check("http://127.0.0.1@evil.com/v1").is_err(),
            "el host real es evil.com"
        );
    }

    #[test]
    fn esquemas_raros_y_vacios_se_rechazan_con_motivo() {
        assert!(check("").is_err());
        assert!(check("   ").is_err());
        assert!(check("localhost:11434").is_err(), "sin esquema no vale");
        assert!(check("ftp://host/v1").is_err());
        assert!(check("file:///etc/passwd").is_err());
        assert!(check("apple-intelligence://local").is_err());
    }

    #[test]
    fn el_esquema_no_distingue_mayusculas() {
        assert!(check("HTTPS://api.openai.com/v1").is_ok());
        assert!(check("HtTp://LOCALHOST:11434/v1").is_ok());
    }

    #[test]
    fn el_default_de_fabrica_del_proveedor_custom_pasa_su_propia_validacion() {
        // Si el default no pasara, la app arrancaria con un valor que ella
        // misma rechaza al guardarlo.
        let providers = super::default_post_process_providers();
        let custom = providers
            .iter()
            .find(|p| p.id == "custom")
            .expect("el proveedor custom tiene que existir");
        assert!(
            check(&custom.base_url).is_ok(),
            "el default de fabrica no pasa la validacion: {}",
            custom.base_url
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_settings_disable_auto_submit() {
        let settings = get_default_settings();
        assert!(!settings.auto_submit);
        assert_eq!(settings.auto_submit_key, AutoSubmitKey::Enter);
        assert_eq!(
            settings.settings_schema_version,
            CURRENT_SETTINGS_SCHEMA_VERSION
        );
    }

    #[test]
    fn the_formalize_binding_ships_by_default() {
        let settings = get_default_settings();

        let binding = settings
            .bindings
            .get("transcribe_and_formalize")
            .expect("el atajo de formalizar debe venir de fabrica");

        assert!(!binding.current_binding.is_empty());
        assert_eq!(binding.current_binding, binding.default_binding);
    }

    #[test]
    fn the_formalize_default_is_a_single_key_and_never_ctrl_alt() {
        // En teclado espanol AltGr ES Ctrl+Alt: un default con esa combinacion
        // se dispararia al escribir @, #, € o \. Y nada de acordes de tres
        // teclas sostenidas. Ver el spec y modifiers.rs:128, donde alt_right
        // esta aliaseado a "altgr".
        let settings = get_default_settings();
        let binding = &settings.bindings["transcribe_and_formalize"].default_binding;

        assert!(
            !binding.contains('+'),
            "el default debe ser una sola tecla, es {binding}"
        );
        assert!(
            !binding.contains("alt_right") && !binding.contains("altgr"),
            "AltGr jamas, es {binding}"
        );
        assert!(
            !binding.contains("shift_right"),
            "shift_right se pisa con cada mayuscula, es {binding}"
        );
    }

    /// Ronda de arreglo final (hallazgo 3+4): un modificador desnudo (`ctrl_right`
    /// en Windows/Linux, `cmd_right` en macOS) traga la propia PULSACION cuando el
    /// estado resultante coincide con el hotkey (Ctrl+C dispara la grabacion y la
    /// app enfocada recibe una "c" suelta), y bajo la implementacion Tauri (la de
    /// por defecto en Linux) `"ctrl_right".parse::<Shortcut>()` directamente falla
    /// tras pasar la validacion, dejando el atajo mudo en silencio. El test de
    /// forma de arriba no distingue este default de un typo como "ctrl_l"; este
    /// fija el literal para las tres plataformas.
    #[test]
    fn the_formalize_default_shortcut_is_f9_on_every_platform() {
        let settings = get_default_settings();
        let binding = &settings.bindings["transcribe_and_formalize"];

        assert_eq!(
            binding.default_binding, "f9",
            "el default debe ser f9 en toda plataforma, es {}",
            binding.default_binding
        );
        assert_eq!(binding.current_binding, "f9");
    }

    #[cfg(not(target_os = "linux"))]
    #[test]
    fn default_overlay_style_is_live_when_overlay_defaults_on() {
        let settings = get_default_settings();
        assert_eq!(settings.overlay_style, OverlayStyle::Live);
    }

    #[test]
    fn overlay_migration_keeps_disabled_overlay_off() {
        let mut settings = get_default_settings();

        // Legacy store: overlay was hidden via the retired position "none".
        let raw = serde_json::json!({
            "selected_model": "",
            "overlay_position": "none"
        });

        assert!(apply_settings_migrations(&mut settings, &raw));
        assert_eq!(settings.overlay_style, OverlayStyle::None);
    }

    #[test]
    fn legacy_none_overlay_position_deserializes_to_bottom() {
        // A persisted "none" must not fail the whole settings load; the serde
        // alias folds it onto Bottom (visibility is owned by overlay_style).
        let raw = serde_json::json!({ "overlay_position": "none" });
        let position: OverlayPosition =
            serde_json::from_value(raw.get("overlay_position").unwrap().clone())
                .expect("legacy \"none\" should deserialize, not error");
        assert_eq!(position, OverlayPosition::Bottom);
    }

    #[test]
    fn overlay_migration_promotes_enabled_overlay_to_live() {
        let mut settings = get_default_settings();
        settings.overlay_position = OverlayPosition::Top;
        settings.overlay_style = OverlayStyle::Minimal;

        let raw = serde_json::json!({
            "selected_model": "",
            "overlay_position": "top"
        });

        assert!(apply_settings_migrations(&mut settings, &raw));
        assert_eq!(settings.overlay_style, OverlayStyle::Live);
        assert_eq!(settings.overlay_position, OverlayPosition::Top);
    }

    #[test]
    fn gpu_device_migration_resets_legacy_positive_selection_to_auto() {
        let mut settings = get_default_settings();
        settings.transcribe_accelerator = TranscribeAcceleratorSetting::Gpu;
        settings.transcribe_gpu_device = 2;

        let raw = serde_json::json!({
            "transcribe_accelerator": "gpu",
            "transcribe_gpu_device": 2
        });

        assert!(apply_settings_migrations(&mut settings, &raw));
        assert_eq!(
            settings.transcribe_accelerator,
            TranscribeAcceleratorSetting::Auto
        );
        assert_eq!(
            settings.transcribe_gpu_device,
            default_transcribe_gpu_device()
        );
        assert_eq!(
            settings.settings_schema_version,
            CURRENT_SETTINGS_SCHEMA_VERSION
        );
    }

    #[test]
    fn gpu_device_migration_keeps_current_schema_positive_selection() {
        let mut settings = get_default_settings();
        settings.transcribe_accelerator = TranscribeAcceleratorSetting::Gpu;
        settings.transcribe_gpu_device = 2;

        let raw = serde_json::json!({
            "settings_schema_version": CURRENT_SETTINGS_SCHEMA_VERSION,
            "onboarding_completed": false,
            "whats_new_last_seen_version": default_whats_new_last_seen_version(),
            "overlay_style": "live",
            "transcribe_accelerator": "gpu",
            "transcribe_gpu_device": 2
        });

        assert!(!apply_settings_migrations(&mut settings, &raw));
        assert_eq!(
            settings.transcribe_accelerator,
            TranscribeAcceleratorSetting::Gpu
        );
        assert_eq!(settings.transcribe_gpu_device, 2);
    }

    #[test]
    fn debug_output_redacts_api_keys() {
        let mut settings = get_default_settings();
        settings
            .post_process_api_keys
            .insert("openai".to_string(), "sk-proj-secret-key-12345".to_string());
        settings.post_process_api_keys.insert(
            "anthropic".to_string(),
            "sk-ant-secret-key-67890".to_string(),
        );
        settings
            .post_process_api_keys
            .insert("empty_provider".to_string(), "".to_string());

        let debug_output = format!("{:?}", settings);

        assert!(!debug_output.contains("sk-proj-secret-key-12345"));
        assert!(!debug_output.contains("sk-ant-secret-key-67890"));
        assert!(debug_output.contains("[REDACTED]"));
    }

    #[test]
    fn secret_map_debug_redacts_values() {
        let map = SecretMap(HashMap::from([("key".into(), "secret".into())]));
        let out = format!("{:?}", map);
        assert!(!out.contains("secret"));
        assert!(out.contains("[REDACTED]"));
    }

    #[test]
    fn default_clipboard_handling_keeps_transcript_on_clipboard() {
        // Safety net: a silent paste failure cannot be detected, so the only
        // way to guarantee a dictation is never lost is to leave it on the
        // clipboard by default.
        assert_eq!(
            get_default_settings().clipboard_handling,
            ClipboardHandling::CopyToClipboard
        );
    }

    #[test]
    fn migration_v2_switches_clipboard_handling_to_copy_to_clipboard() {
        let mut settings = get_default_settings();
        settings.clipboard_handling = ClipboardHandling::DontModify;
        settings.settings_schema_version = 1;

        let raw = serde_json::json!({
            "settings_schema_version": 1,
            "onboarding_completed": true,
            "whats_new_last_seen_version": default_whats_new_last_seen_version(),
            "overlay_style": "live",
            "clipboard_handling": "dont_modify",
        });

        assert!(apply_settings_migrations(&mut settings, &raw));
        assert_eq!(
            settings.clipboard_handling,
            ClipboardHandling::CopyToClipboard
        );
    }

    #[test]
    fn migration_v8_repairs_clipboard_handling_flipped_by_the_ui() {
        // La interfaz proponía `dont_modify` mientras los ajustes no habían
        // cargado, y el desplegable escribía al pulsar la opción ya marcada, así
        // que había stores con `dont_modify` que nadie eligió. Un store ya en v7
        // está fuera del alcance de la v3, de modo que sin este paso quedaría
        // roto para siempre.
        let mut settings = get_default_settings();
        settings.clipboard_handling = ClipboardHandling::DontModify;
        settings.settings_schema_version = 7;

        let raw = serde_json::json!({
            "settings_schema_version": 7,
            "onboarding_completed": true,
            "whats_new_last_seen_version": default_whats_new_last_seen_version(),
            "overlay_style": "live",
            "clipboard_handling": "dont_modify",
        });

        assert!(apply_settings_migrations(&mut settings, &raw));
        assert_eq!(
            settings.clipboard_handling,
            ClipboardHandling::CopyToClipboard,
            "un store en v7 con dont_modify debe repararse"
        );
        assert_eq!(
            settings.settings_schema_version,
            CURRENT_SETTINGS_SCHEMA_VERSION
        );
    }

    #[test]
    fn migration_v8_runs_only_once() {
        // El precio aceptado: la v8 pisa también a quien eligió `dont_modify` a
        // propósito. Lo que no puede hacer es pisárselo DOS veces — tras volver
        // a seleccionarlo, su elección tiene que sobrevivir a cada arranque.
        let mut settings = get_default_settings();
        settings.clipboard_handling = ClipboardHandling::DontModify;
        settings.settings_schema_version = CURRENT_SETTINGS_SCHEMA_VERSION;

        let raw = serde_json::json!({
            "settings_schema_version": CURRENT_SETTINGS_SCHEMA_VERSION,
            "onboarding_completed": true,
            "whats_new_last_seen_version": default_whats_new_last_seen_version(),
            "overlay_style": "live",
            "clipboard_handling": "dont_modify",
        });

        assert!(!apply_settings_migrations(&mut settings, &raw));
        assert_eq!(settings.clipboard_handling, ClipboardHandling::DontModify);
    }

    #[test]
    fn clipboard_migration_applies_to_stores_already_at_v2() {
        // A dev-machine store can have been bumped to v2 (prompts seeded) by an
        // intermediate build that predated the clipboard flip; the flip must
        // still apply to it, which is why it lives in its own schema step.
        let mut settings = get_default_settings();
        settings.clipboard_handling = ClipboardHandling::DontModify;
        settings.settings_schema_version = 2;

        let raw = serde_json::json!({
            "settings_schema_version": 2,
            "onboarding_completed": true,
            "whats_new_last_seen_version": default_whats_new_last_seen_version(),
            "overlay_style": "live",
            "clipboard_handling": "dont_modify",
        });

        assert!(apply_settings_migrations(&mut settings, &raw));
        assert_eq!(
            settings.clipboard_handling,
            ClipboardHandling::CopyToClipboard
        );
        assert_eq!(
            settings.settings_schema_version,
            CURRENT_SETTINGS_SCHEMA_VERSION
        );
    }

    /// Five entries is not enough to hold a test corpus: three separate
    /// 8-15 dictation corpora were auto-deleted mid-evaluation in July 2026,
    /// one of them while its recordings were being copied out.
    /// Adding the microphone gain must not change what an existing user hears:
    /// a store written before this setting existed has to come back at unity,
    /// not at some "helpful" boost.
    /// Reported 2026-07-28 on a multi-channel Realtek device ("Varios
    /// micrófonos (2- Realtek Audio)", 48 kHz, 2 channels, Windows 11): with
    /// the VAD on, a 13 s dictation reached the model as 1.05-2.16 s of audio.
    /// The recordings on disk are already short, so frames are being dropped
    /// during capture, not during decode.
    ///
    /// Until the root cause is understood, losing the VAD's silence trimming is
    /// far cheaper than losing 90% of every dictation, so the shipped default
    /// is off.
    /// The default change alone protects nobody who already installed Trazo:
    /// their store holds `vad_enabled: true` and serde never touches a field
    /// that is present. The migration is the part that actually reaches the
    /// machines currently losing dictations.
    #[test]
    fn vad_migration_turns_off_a_stored_enabled_vad() {
        let mut settings = get_default_settings();
        settings.vad_enabled = true;
        settings.settings_schema_version = 5;

        let stored = serde_json::json!({ "settings_schema_version": 5, "vad_enabled": true });
        let updated = apply_settings_migrations(&mut settings, &stored);

        assert!(
            updated,
            "the migration must report that it changed something"
        );
        assert!(
            !settings.vad_enabled,
            "a store carrying the old enabled-by-default VAD must be turned off"
        );
        assert_eq!(
            settings.settings_schema_version,
            CURRENT_SETTINGS_SCHEMA_VERSION
        );
    }

    /// Runs once. Someone who deliberately switches the VAD back on after the
    /// migration must keep it on across restarts.
    #[test]
    fn vad_migration_does_not_run_twice() {
        let mut settings = get_default_settings();
        settings.vad_enabled = true;
        settings.settings_schema_version = CURRENT_SETTINGS_SCHEMA_VERSION;

        let stored = serde_json::json!({
            "settings_schema_version": CURRENT_SETTINGS_SCHEMA_VERSION,
            "vad_enabled": true
        });
        apply_settings_migrations(&mut settings, &stored);

        assert!(
            settings.vad_enabled,
            "a deliberate re-enable after the migration must survive"
        );
    }

    #[test]
    fn vad_is_off_by_default_until_the_capture_truncation_is_understood() {
        assert!(
            !get_default_settings().vad_enabled,
            "the VAD default must stay off while it can silently eat a dictation"
        );
    }

    #[test]
    fn microphone_gain_defaults_to_unity() {
        assert_eq!(
            get_default_settings().microphone_gain,
            crate::audio_toolkit::UNITY_GAIN,
            "a fresh install must not silently amplify the microphone"
        );
    }

    /// The slider cannot produce these, but a hand-edited store can, and a
    /// zero would look exactly like a dead microphone.
    #[test]
    fn microphone_gain_outside_the_offered_range_is_clamped() {
        for (stored, expected) in [
            (0.0f32, crate::audio_toolkit::MIN_GAIN),
            (99.0, crate::audio_toolkit::MAX_GAIN),
            (-3.0, crate::audio_toolkit::MIN_GAIN),
        ] {
            assert_eq!(
                sanitize_microphone_gain(stored),
                expected,
                "stored gain {stored} must be clamped into the usable range"
            );
        }
    }

    #[test]
    fn microphone_gain_inside_the_range_is_left_alone() {
        assert_eq!(sanitize_microphone_gain(2.5), 2.5);
    }

    #[test]
    fn default_history_limit_holds_a_test_corpus() {
        assert!(
            get_default_settings().history_limit >= 15,
            "the default must survive a 10-15 recording corpus, got {}",
            get_default_settings().history_limit
        );
    }

    #[test]
    fn history_limit_migration_raises_the_untouched_legacy_default() {
        let mut settings = get_default_settings();
        settings.history_limit = LEGACY_HISTORY_LIMIT;
        settings.settings_schema_version = 4;

        let raw = serde_json::json!({
            "settings_schema_version": 4,
            "onboarding_completed": true,
            "whats_new_last_seen_version": default_whats_new_last_seen_version(),
            "overlay_style": "live",
            "history_limit": LEGACY_HISTORY_LIMIT,
        });

        assert!(apply_settings_migrations(&mut settings, &raw));
        assert_eq!(settings.history_limit, default_history_limit());
        assert_eq!(
            settings.settings_schema_version,
            CURRENT_SETTINGS_SCHEMA_VERSION
        );
    }

    #[test]
    fn history_limit_migration_keeps_a_deliberate_choice() {
        for chosen in [3usize, 50, 200] {
            let mut settings = get_default_settings();
            settings.history_limit = chosen;
            settings.settings_schema_version = 4;

            let raw = serde_json::json!({
                "settings_schema_version": 4,
                "onboarding_completed": true,
                "whats_new_last_seen_version": default_whats_new_last_seen_version(),
                "overlay_style": "live",
                "history_limit": chosen,
            });

            apply_settings_migrations(&mut settings, &raw);
            assert_eq!(
                settings.history_limit, chosen,
                "a history limit the user picked must survive the migration"
            );
        }
    }

    #[test]
    fn default_recording_volume_is_disabled() {
        assert_eq!(get_default_settings().recording_volume, None);
    }

    #[test]
    fn mute_migration_converts_mute_to_recording_volume_zero() {
        let mut settings = get_default_settings();
        settings.mute_while_recording = true;
        settings.recording_volume = None;
        settings.settings_schema_version = 3;

        let raw = serde_json::json!({
            "settings_schema_version": 3,
            "onboarding_completed": true,
            "whats_new_last_seen_version": default_whats_new_last_seen_version(),
            "overlay_style": "live",
            "mute_while_recording": true,
        });

        assert!(apply_settings_migrations(&mut settings, &raw));
        assert_eq!(settings.recording_volume, Some(0.0));
        assert!(
            !settings.mute_while_recording,
            "legacy flag must be turned off so muting is not applied twice"
        );
        assert_eq!(
            settings.settings_schema_version,
            CURRENT_SETTINGS_SCHEMA_VERSION
        );
    }

    #[test]
    fn mute_migration_leaves_disabled_mute_alone() {
        let mut settings = get_default_settings();
        settings.mute_while_recording = false;
        settings.recording_volume = None;
        settings.settings_schema_version = 3;

        let raw = serde_json::json!({
            "settings_schema_version": 3,
            "onboarding_completed": true,
            "whats_new_last_seen_version": default_whats_new_last_seen_version(),
            "overlay_style": "live",
            "mute_while_recording": false,
        });

        apply_settings_migrations(&mut settings, &raw);
        assert_eq!(settings.recording_volume, None);
    }

    #[test]
    fn mute_migration_keeps_existing_recording_volume() {
        let mut settings = get_default_settings();
        settings.mute_while_recording = true;
        settings.recording_volume = Some(0.3);
        settings.settings_schema_version = 3;

        let raw = serde_json::json!({
            "settings_schema_version": 3,
            "onboarding_completed": true,
            "whats_new_last_seen_version": default_whats_new_last_seen_version(),
            "overlay_style": "live",
            "mute_while_recording": true,
        });

        apply_settings_migrations(&mut settings, &raw);
        assert_eq!(
            settings.recording_volume,
            Some(0.3),
            "an explicit duck level must not be overwritten by the mute migration"
        );
    }

    #[test]
    fn current_schema_respects_dont_modify_choice() {
        let mut settings = get_default_settings();
        settings.clipboard_handling = ClipboardHandling::DontModify;

        let raw = serde_json::json!({
            "settings_schema_version": CURRENT_SETTINGS_SCHEMA_VERSION,
            "onboarding_completed": true,
            "whats_new_last_seen_version": default_whats_new_last_seen_version(),
            "overlay_style": "live",
            "clipboard_handling": "dont_modify",
        });

        assert!(!apply_settings_migrations(&mut settings, &raw));
        assert_eq!(settings.clipboard_handling, ClipboardHandling::DontModify);
    }

    const SPANISH_PROFILE_IDS: [&str; 3] = [
        "default_es_casual",
        "default_es_commit",
        "default_es_community",
    ];

    #[test]
    fn default_prompts_include_spanish_profiles() {
        let prompts = default_post_process_prompts();
        for id in SPANISH_PROFILE_IDS {
            let prompt = prompts
                .iter()
                .find(|p| p.id == id)
                .unwrap_or_else(|| panic!("default prompts must include '{id}'"));
            // Legacy (non-structured-output) mode inserts the transcript via
            // this placeholder; without it the transcript never reaches the LLM.
            assert!(
                prompt.prompt.contains("${output}"),
                "profile '{id}' must contain the ${{output}} placeholder"
            );
        }
        assert_eq!(
            get_default_settings()
                .post_process_selected_prompt_id
                .as_deref(),
            Some("default_es_casual"),
            "fresh installs must have a selected prompt so post-processing is not silently skipped"
        );
    }

    #[test]
    fn prompt_migration_appends_spanish_profiles_and_selects_one() {
        let mut settings = get_default_settings();
        settings.post_process_prompts = vec![LLMPrompt {
            id: "default_improve_transcriptions".to_string(),
            name: "Improve Transcriptions".to_string(),
            prompt: "old prompt ${output}".to_string(),
        }];
        settings.post_process_selected_prompt_id = None;
        settings.settings_schema_version = 1;

        let raw = serde_json::json!({
            "settings_schema_version": 1,
            "onboarding_completed": true,
            "whats_new_last_seen_version": default_whats_new_last_seen_version(),
            "overlay_style": "live",
        });

        assert!(apply_settings_migrations(&mut settings, &raw));
        for id in SPANISH_PROFILE_IDS {
            assert!(
                settings.post_process_prompts.iter().any(|p| p.id == id),
                "migration must append missing profile '{id}'"
            );
        }
        assert_eq!(
            settings.post_process_selected_prompt_id.as_deref(),
            Some("default_es_casual"),
            "migration must select a prompt when none was selected"
        );
        assert_eq!(
            settings.settings_schema_version,
            CURRENT_SETTINGS_SCHEMA_VERSION
        );
    }

    #[test]
    fn prompt_migration_preserves_user_prompts_and_selection() {
        let mut settings = get_default_settings();
        settings.post_process_prompts = vec![
            LLMPrompt {
                id: "default_es_casual".to_string(),
                name: "Mi casual editado".to_string(),
                prompt: "texto editado por el usuario ${output}".to_string(),
            },
            LLMPrompt {
                id: "my_custom".to_string(),
                name: "Custom".to_string(),
                prompt: "custom ${output}".to_string(),
            },
        ];
        settings.post_process_selected_prompt_id = Some("my_custom".to_string());
        settings.settings_schema_version = 1;

        let raw = serde_json::json!({
            "settings_schema_version": 1,
            "onboarding_completed": true,
            "whats_new_last_seen_version": default_whats_new_last_seen_version(),
            "overlay_style": "live",
        });

        apply_settings_migrations(&mut settings, &raw);
        let casual_count = settings
            .post_process_prompts
            .iter()
            .filter(|p| p.id == "default_es_casual")
            .count();
        assert_eq!(
            casual_count, 1,
            "migration must not duplicate an existing id"
        );
        let casual = settings
            .post_process_prompts
            .iter()
            .find(|p| p.id == "default_es_casual")
            .unwrap();
        assert_eq!(
            casual.prompt, "texto editado por el usuario ${output}",
            "migration must not overwrite user-edited prompt text"
        );
        assert_eq!(
            settings.post_process_selected_prompt_id.as_deref(),
            Some("my_custom"),
            "migration must not change an explicit selection"
        );
    }

    #[test]
    fn the_email_profile_ships_with_a_fresh_install() {
        let settings = get_default_settings();

        let email = settings
            .post_process_prompts
            .iter()
            .find(|p| p.id == DEFAULT_EMAIL_PROMPT_ID)
            .expect("el perfil de correo debe venir sembrado");

        // El formalizador es inutil sin sus tres variables.
        assert!(email.prompt.contains("${saludo}"), "falta ${{saludo}}");
        assert!(
            email.prompt.contains("${nombre_usuario}"),
            "falta ${{nombre_usuario}}"
        );
        assert!(
            email.prompt.contains("${tratamiento}"),
            "falta ${{tratamiento}}"
        );
        // Y sin ${output} el post-procesado no recibe la transcripcion.
        assert!(email.prompt.contains("${output}"), "falta ${{output}}");
    }

    #[test]
    fn formalize_defaults_point_at_the_seeded_profile() {
        let settings = get_default_settings();

        assert_eq!(
            settings.formalize_prompt_id.as_deref(),
            Some(DEFAULT_EMAIL_PROMPT_ID)
        );
        assert_eq!(settings.formality_treatment, FormalityTreatment::Tu);
        assert_eq!(settings.user_full_name, "");
    }

    #[test]
    fn v7_migration_seeds_the_email_profile_into_an_existing_store() {
        let mut settings = get_default_settings();
        settings.settings_schema_version = 6;
        settings
            .post_process_prompts
            .retain(|p| p.id != DEFAULT_EMAIL_PROMPT_ID);
        settings.formalize_prompt_id = None;

        let raw = serde_json::json!({
            "settings_schema_version": 6,
            "onboarding_completed": true,
            "whats_new_last_seen_version": default_whats_new_last_seen_version(),
            "overlay_style": "live",
        });

        assert!(apply_settings_migrations(&mut settings, &raw));
        assert!(
            settings
                .post_process_prompts
                .iter()
                .any(|p| p.id == DEFAULT_EMAIL_PROMPT_ID),
            "la migracion debe sembrar el perfil de correo"
        );
        assert_eq!(
            settings.formalize_prompt_id.as_deref(),
            Some(DEFAULT_EMAIL_PROMPT_ID)
        );
        assert_eq!(
            settings.settings_schema_version,
            CURRENT_SETTINGS_SCHEMA_VERSION
        );
    }

    #[test]
    fn v7_migration_never_clobbers_an_edited_email_profile() {
        let mut settings = get_default_settings();
        settings.settings_schema_version = 6;
        for prompt in settings.post_process_prompts.iter_mut() {
            if prompt.id == DEFAULT_EMAIL_PROMPT_ID {
                prompt.prompt = "MI VERSION EDITADA ${output}".to_string();
            }
        }

        let raw = serde_json::json!({
            "settings_schema_version": 6,
            "onboarding_completed": true,
            "whats_new_last_seen_version": default_whats_new_last_seen_version(),
            "overlay_style": "live",
        });

        apply_settings_migrations(&mut settings, &raw);

        let email = settings
            .post_process_prompts
            .iter()
            .find(|p| p.id == DEFAULT_EMAIL_PROMPT_ID)
            .expect("sigue existiendo");
        assert_eq!(
            email.prompt, "MI VERSION EDITADA ${output}",
            "un prompt editado por el usuario nunca se pisa"
        );
    }

    /// Hallazgo 5: a diferencia de la v2 (que siembra los TRES perfiles ES a
    /// la vez, asi que recorrer default_post_process_prompts() completo es lo
    /// correcto ahi), la v7 solo tiene que sembrar el perfil de correo nuevo.
    /// Recorrer la lista entera resucita cualquier perfil que el usuario haya
    /// borrado deliberadamente en v6 (aqui, default_es_commit).
    #[test]
    fn v7_migration_does_not_resurrect_a_deleted_default_profile() {
        let mut settings = get_default_settings();
        settings.settings_schema_version = 6;
        settings
            .post_process_prompts
            .retain(|p| p.id != "default_es_commit" && p.id != DEFAULT_EMAIL_PROMPT_ID);

        let raw = serde_json::json!({
            "settings_schema_version": 6,
            "onboarding_completed": true,
            "whats_new_last_seen_version": default_whats_new_last_seen_version(),
            "overlay_style": "live",
        });

        apply_settings_migrations(&mut settings, &raw);

        assert!(
            !settings
                .post_process_prompts
                .iter()
                .any(|p| p.id == "default_es_commit"),
            "un perfil borrado por el usuario en v6 no debe reaparecer en v7"
        );
        assert!(
            settings
                .post_process_prompts
                .iter()
                .any(|p| p.id == DEFAULT_EMAIL_PROMPT_ID),
            "la v7 si debe sembrar el perfil de correo, que es lo nuevo de este schema"
        );
    }

    #[test]
    fn v7_migration_leaves_the_global_selection_alone() {
        // El atajo de formalizar tiene su propio ajuste; la seleccion global
        // del usuario (su perfil del dia a dia) no se toca.
        let mut settings = get_default_settings();
        settings.settings_schema_version = 6;
        settings.post_process_selected_prompt_id = Some("default_es_casual".to_string());

        let raw = serde_json::json!({
            "settings_schema_version": 6,
            "onboarding_completed": true,
            "whats_new_last_seen_version": default_whats_new_last_seen_version(),
            "overlay_style": "live",
        });

        apply_settings_migrations(&mut settings, &raw);

        assert_eq!(
            settings.post_process_selected_prompt_id.as_deref(),
            Some("default_es_casual")
        );
    }

    /// Hallazgo 6: hasta que exista un comando para fijar `formalize_prompt_id`
    /// (ver `shortcut::set_formalize_prompt`), nadie podia llegar a v7 con ese
    /// campo ya en `Some(custom)`. Ahora que el comando existe, la migracion
    /// nunca debe pisar una eleccion explicita del usuario, igual que ya hace
    /// con `post_process_selected_prompt_id`.
    #[test]
    fn v7_migration_does_not_override_an_already_set_formalize_prompt_id() {
        let mut settings = get_default_settings();
        settings.settings_schema_version = 6;
        settings.formalize_prompt_id = Some("my_custom_email_profile".to_string());

        let raw = serde_json::json!({
            "settings_schema_version": 6,
            "onboarding_completed": true,
            "whats_new_last_seen_version": default_whats_new_last_seen_version(),
            "overlay_style": "live",
        });

        apply_settings_migrations(&mut settings, &raw);

        assert_eq!(
            settings.formalize_prompt_id.as_deref(),
            Some("my_custom_email_profile"),
            "un formalize_prompt_id ya elegido por el usuario nunca se pisa"
        );
    }
}
