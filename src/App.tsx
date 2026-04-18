import { Fragment, lazy, Suspense, useEffect, useRef, useState } from "react";
import { Folder, Play, History, Settings, FileSearch, ShieldAlert, FileIcon, FolderIcon, ArrowUp, ArrowDown, Eye, Plus, X, Sun, Moon, Download, RefreshCw } from "lucide-react";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { ScrollArea } from "@/components/ui/scroll-area";
import { Separator } from "@/components/ui/separator";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from "@/components/ui/table";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";

// Tauri APIs
import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
import { check, type DownloadEvent, type Update } from "@tauri-apps/plugin-updater";

const AnalyticsCharts = lazy(() =>
  import("@/components/analytics-charts").then((module) => ({ default: module.AnalyticsCharts }))
);


type FileNode = {
  name: string;
  path: string;
  is_dir: boolean;
  size: number;
  extension: string | null;
  category?: string;
  suggested_folder?: string;
};

type MoveOperation = {
  original_path: string;
  target_path: string;
  status: string;
  error_msg?: string | null;
};

type TransactionManifest = {
  transaction_id: string;
  root_folder: string;
  moves: MoveOperation[];
  timestamp: string;
};

type RuleConditions = {
  extensions: string[];
  filename_keywords: string[];
  min_size_bytes?: number | null;
  max_size_bytes?: number | null;
};

type ClassificationRule = {
  id: string;
  name: string;
  category_path: string;
  destination_folder: string;
  priority: number;
  enabled: boolean;
  action: "move" | "copy" | "delete" | "ignore";
  conditions: RuleConditions;
};

type RuleConfig = {
  version: number;
  stop_on_match: boolean;
  unknown_folder: string;
  rules: ClassificationRule[];
};

type NewRuleDraft = {
  id: string;
  name: string;
  destination_folder: string;
  category_path: string;
  action: "move" | "copy" | "delete" | "ignore";
  extensions: string;
  keywords: string;
};

type AIProvider = "gemini" | "openai" | "anthropic" | "ollama" | "openrouter";

type AISettings = {
  version: number;
  enabled: boolean;
  ai_first_with_fallback: boolean;
  complete_ai_sorting: boolean;
  selected_provider: AIProvider;
  selected_model: string;
  custom_base_url?: string | null;
};

type AISettingsEnvelope = {
  settings: AISettings;
  api_key_present: boolean;
};

type SaveAISettingsRequest = {
  settings: AISettings;
  api_key?: string | null;
};

type ProviderRequest = {
  provider?: AIProvider;
  api_key?: string | null;
  base_url?: string | null;
};

type ProviderValidationResult = {
  available: boolean;
  message: string;
};

type ScanOptions = {
  enable_ai?: boolean;
  ai_first_with_fallback?: boolean;
  complete_ai_sorting?: boolean;
};

type ScanMetrics = {
  files_seen: number;
  files_classified_by_rules: number;
  files_classified_by_ai: number;
  files_unknown: number;
};

type ScanResponse = {
  files: FileNode[];
  metrics: ScanMetrics;
};

type ThemeMode = "dark" | "light";

type UpdatePhase =
  | "idle"
  | "checking"
  | "up-to-date"
  | "downloading"
  | "ready-to-install"
  | "installing"
  | "installed"
  | "error";

type UpdateStatus = {
  phase: UpdatePhase;
  message: string;
  version?: string;
  downloadedBytes?: number;
  totalBytes?: number;
};

const PROVIDER_OPTIONS: Array<{ value: AIProvider; label: string }> = [
  { value: "gemini", label: "Gemini" },
  { value: "openai", label: "OpenAI" },
  { value: "anthropic", label: "Anthropic" },
  { value: "ollama", label: "Ollama" },
  { value: "openrouter", label: "OpenRouter-Compatible" },
];

const DEFAULT_MODEL_BY_PROVIDER: Record<AIProvider, string> = {
  gemini: "gemini-2.5-flash",
  openai: "gpt-4o-mini",
  anthropic: "claude-3-5-sonnet-latest",
  ollama: "llama3.1",
  openrouter: "openai/gpt-4o-mini",
};

const THEME_STORAGE_KEY = "ordinex-theme";

const providerNeedsBaseUrl = (provider: AIProvider) => provider === "ollama" || provider === "openrouter";

const defaultAISettings = (): AISettings => ({
  version: 1,
  enabled: true,
  ai_first_with_fallback: true,
  complete_ai_sorting: false,
  selected_provider: "gemini",
  selected_model: DEFAULT_MODEL_BY_PROVIDER.gemini,
  custom_base_url: null,
});

export default function App() {
  const [selectedPath, setSelectedPath] = useState<string | null>(null);
  const [files, setFiles] = useState<FileNode[]>([]);
  const [scanning, setScanning] = useState(false);
  const [executing, setExecuting] = useState(false);
  const [currentView, setCurrentView] = useState<"dashboard" | "history" | "rules">("dashboard");
  const [history, setHistory] = useState<TransactionManifest[]>([]);
  const [ruleConfig, setRuleConfig] = useState<RuleConfig | null>(null);
  const [rulesLoading, setRulesLoading] = useState(false);
  const [rulesSaving, setRulesSaving] = useState(false);
  const [rulePreview, setRulePreview] = useState<Record<string, string[]>>({});
  const [ruleTokenDrafts, setRuleTokenDrafts] = useState<Record<string, string>>({});
  const [newRuleDraft, setNewRuleDraft] = useState<NewRuleDraft>({
    id: "",
    name: "",
    destination_folder: "",
    category_path: "",
    action: "move",
    extensions: "",
    keywords: "",
  });
  const [previewLoadingRuleId, setPreviewLoadingRuleId] = useState<string | null>(null);
  const [statusMessage, setStatusMessage] = useState<string | null>(null);
  const [errorMessage, setErrorMessage] = useState<string | null>(null);
  const [aiSettingsEnvelope, setAISettingsEnvelope] = useState<AISettingsEnvelope | null>(null);
  const [aiSettingsDraft, setAISettingsDraft] = useState<AISettings>(defaultAISettings());
  const [aiSettingsDialogOpen, setAISettingsDialogOpen] = useState(false);
  const [aiApiKeyDraft, setAIApiKeyDraft] = useState("");
  const [aiLoading, setAILoading] = useState(false);
  const [aiSaving, setAISaving] = useState(false);
  const [aiModelsLoading, setAIModelsLoading] = useState(false);
  const [aiModels, setAIModels] = useState<string[]>([]);
  const [aiAvailable, setAIAvailable] = useState(false);
  const [aiStatusMessage, setAIStatusMessage] = useState("AI provider is not validated.");
  const [themeMode, setThemeMode] = useState<ThemeMode>(() =>
    document.documentElement.classList.contains("dark") ? "dark" : "light"
  );
  const [updateStatus, setUpdateStatus] = useState<UpdateStatus>({
    phase: "idle",
    message: "Update check has not started.",
  });
  const pendingUpdateRef = useRef<Update | null>(null);

  const applyThemeMode = (mode: ThemeMode) => {
    document.documentElement.classList.toggle("dark", mode === "dark");
    localStorage.setItem(THEME_STORAGE_KEY, mode);
    setThemeMode(mode);
  };

  const toggleThemeMode = () => {
    applyThemeMode(themeMode === "dark" ? "light" : "dark");
  };

  const autoCheckAndDownloadUpdate = async () => {
    if (
      updateStatus.phase === "checking" ||
      updateStatus.phase === "downloading" ||
      updateStatus.phase === "installing"
    ) {
      return;
    }

    setUpdateStatus({
      phase: "checking",
      message: "Checking for updates...",
    });

    try {
      const update = await check();
      if (!update) {
        setUpdateStatus({
          phase: "up-to-date",
          message: "You are running the latest version.",
        });
        return;
      }

      if (pendingUpdateRef.current) {
        await pendingUpdateRef.current.close().catch(() => undefined);
      }
      pendingUpdateRef.current = update;

      let downloadedBytes = 0;
      let totalBytes = 0;

      setUpdateStatus({
        phase: "downloading",
        message: `Update ${update.version} found. Downloading in background...`,
        version: update.version,
      });

      await update.download((event: DownloadEvent) => {
        if (event.event === "Started") {
          totalBytes = event.data.contentLength ?? 0;
        } else if (event.event === "Progress") {
          downloadedBytes += event.data.chunkLength;
          setUpdateStatus((prev) => ({
            ...prev,
            phase: "downloading",
            downloadedBytes,
            totalBytes,
            message:
              totalBytes > 0
                ? `Downloading update: ${Math.round((downloadedBytes / totalBytes) * 100)}%`
                : "Downloading update...",
          }));
        } else if (event.event === "Finished") {
          setUpdateStatus((prev) => ({
            ...prev,
            phase: "ready-to-install",
            message: "Update downloaded. Install when ready.",
            downloadedBytes,
            totalBytes,
          }));
        }
      });

      setUpdateStatus((prev) => ({
        ...prev,
        phase: "ready-to-install",
        message: "Update downloaded. Install when ready.",
        downloadedBytes,
        totalBytes,
      }));
    } catch (error) {
      const message = error instanceof Error ? error.message : String(error);
      setUpdateStatus({
        phase: "error",
        message: `Updater unavailable: ${message}`,
      });
    }
  };

  const installDownloadedUpdate = async () => {
    if (!pendingUpdateRef.current) {
      setUpdateStatus((prev) => ({
        ...prev,
        phase: "error",
        message: "No downloaded update is ready to install.",
      }));
      return;
    }

    try {
      setUpdateStatus((prev) => ({
        ...prev,
        phase: "installing",
        message: "Installing update...",
      }));

      await pendingUpdateRef.current.install();
      await pendingUpdateRef.current.close().catch(() => undefined);
      pendingUpdateRef.current = null;

      setUpdateStatus((prev) => ({
        ...prev,
        phase: "installed",
        message: "Update installed. Restart the app to finish applying changes.",
      }));
    } catch (error) {
      const message = error instanceof Error ? error.message : String(error);
      setUpdateStatus((prev) => ({
        ...prev,
        phase: "error",
        message: `Update install failed: ${message}`,
      }));
    }
  };

  useEffect(() => {
    loadRuleConfig();
    loadAISettings();

    void autoCheckAndDownloadUpdate();

    return () => {
      if (pendingUpdateRef.current) {
        void pendingUpdateRef.current.close().catch(() => undefined);
        pendingUpdateRef.current = null;
      }
    };
  }, []);

  const handleSelectFolder = async () => {
    try {
      const selected = await open({
        directory: true,
        multiple: false,
        title: "Select a folder to organize"
      });
      if (selected && typeof selected === "string") {
        setErrorMessage(null);
        setStatusMessage(null);
        setSelectedPath(selected);
        loadDirectory(selected);
        setCurrentView("dashboard");
      }
    } catch (error) {
      console.error("Failed to open dialog:", error);
      const message = error instanceof Error ? error.message : String(error);
      setErrorMessage(`Could not open folder picker. ${message}`);
    }
  };

  const loadDirectory = async (path: string) => {
    setScanning(true);
    setErrorMessage(null);
    try {
      const options: ScanOptions = {
        enable_ai: aiSettingsDraft.enabled,
        ai_first_with_fallback: aiSettingsDraft.ai_first_with_fallback,
        complete_ai_sorting: aiSettingsDraft.complete_ai_sorting,
      };
      const result = await invoke<ScanResponse>("scan_directory_advanced", { path, options });
      const strategyLabel = !aiSettingsDraft.enabled
        ? "rules only"
        : aiSettingsDraft.complete_ai_sorting
          ? "complete AI sorting (no rule fallback)"
        : aiSettingsDraft.ai_first_with_fallback
          ? "AI-first with rules fallback"
          : "rules-first with AI fallback";
      setFiles(result.files);
      setStatusMessage(
        `Scan complete: ${result.metrics.files_seen} files, ${result.metrics.files_classified_by_ai} AI, ${result.metrics.files_classified_by_rules} rules (${strategyLabel}).`
      );
    } catch (e) {
      console.error(e);
      setErrorMessage("Directory scan failed. Please try again.");
    } finally {
      setScanning(false);
    }
  };

  const loadHistory = async () => {
    try {
      const result = await invoke<TransactionManifest[]>("fetch_history");
      setHistory(result);
    } catch (e) {
      console.error("Could not load history", e);
      setErrorMessage("Could not load action history.");
    }
  };

  const loadRuleConfig = async () => {
    setRulesLoading(true);
    try {
      const config = await invoke<RuleConfig>("get_rule_config");
      const sortedRules = [...config.rules].sort((a, b) => a.priority - b.priority);
      setRuleConfig({ ...config, rules: sortedRules });
    } catch (e) {
      console.error("Could not load rule config", e);
      setErrorMessage("Could not load rule configuration.");
    } finally {
      setRulesLoading(false);
    }
  };

  const validateSelectedProvider = async (settings: AISettings, apiKeyOverride?: string | null) => {
    const request: ProviderRequest = {
      provider: settings.selected_provider,
      api_key: apiKeyOverride ?? null,
      base_url: providerNeedsBaseUrl(settings.selected_provider)
        ? settings.custom_base_url || null
        : null,
    };
    const validation = await invoke<ProviderValidationResult>("validate_ai_provider", { request });
    setAIAvailable(validation.available);
    setAIStatusMessage(validation.message);
  };

  const loadModelsForProvider = async (
    settings: AISettings,
    apiKeyOverride?: string | null,
    updateSelectedModel = true
  ) => {
    setAIModelsLoading(true);
    try {
      const request: ProviderRequest = {
        provider: settings.selected_provider,
        api_key: apiKeyOverride ?? null,
        base_url: providerNeedsBaseUrl(settings.selected_provider)
          ? settings.custom_base_url || null
          : null,
      };
      const models = await invoke<string[]>("list_ai_models", { request });
      setAIModels(models);

      if (updateSelectedModel && models.length > 0 && !models.includes(settings.selected_model)) {
        setAISettingsDraft((prev) => ({
          ...prev,
          selected_model: models[0],
        }));
      }
    } catch (e) {
      console.error("Failed to load provider models", e);
      setAIModels([]);
    } finally {
      setAIModelsLoading(false);
    }
  };

  const loadAISettings = async () => {
    setAILoading(true);
    try {
      const envelope = await invoke<AISettingsEnvelope>("get_ai_settings");
      const normalized = {
        ...defaultAISettings(),
        ...envelope.settings,
        selected_model:
          envelope.settings.selected_model || DEFAULT_MODEL_BY_PROVIDER[envelope.settings.selected_provider],
      };
      setAISettingsEnvelope(envelope);
      setAISettingsDraft(normalized);
      setAIApiKeyDraft("");

      await validateSelectedProvider(normalized);
      await loadModelsForProvider(normalized, null, false);
    } catch (e) {
      console.error("Could not load AI settings", e);
      setErrorMessage("Could not load AI settings.");
      setAIAvailable(false);
      setAIStatusMessage("AI provider is not configured.");
    } finally {
      setAILoading(false);
    }
  };

  const handleProviderChange = (provider: AIProvider) => {
    setAISettingsDraft((prev) => ({
      ...prev,
      selected_provider: provider,
      selected_model: DEFAULT_MODEL_BY_PROVIDER[provider],
      custom_base_url: providerNeedsBaseUrl(provider) ? prev.custom_base_url || null : null,
    }));
    setAIApiKeyDraft("");
    setAIModels([]);
    setAIAvailable(false);
    setAIStatusMessage("Provider changed. Validate this provider to update status.");
  };

  const validateAndLoadModels = async () => {
    const keyOverride = aiApiKeyDraft.trim() ? aiApiKeyDraft.trim() : null;
    await validateSelectedProvider(aiSettingsDraft, keyOverride);
    await loadModelsForProvider(aiSettingsDraft, keyOverride, true);
  };

  const saveAISettings = async () => {
    setAISaving(true);
    setErrorMessage(null);
    try {
      const request: SaveAISettingsRequest = {
        settings: aiSettingsDraft,
        api_key: aiApiKeyDraft.trim() ? aiApiKeyDraft.trim() : null,
      };
      const saved = await invoke<AISettingsEnvelope>("save_ai_settings_cmd", { request });
      setAISettingsEnvelope(saved);
      setAIApiKeyDraft("");
      await validateSelectedProvider(aiSettingsDraft);
      await loadModelsForProvider(aiSettingsDraft, null, true);
      setStatusMessage("AI settings saved successfully.");
    } catch (e) {
      console.error("Failed to save AI settings", e);
      setErrorMessage("Failed to save AI settings.");
    } finally {
      setAISaving(false);
    }
  };

  const updateRule = (ruleId: string, updater: (r: ClassificationRule) => ClassificationRule) => {
    setRuleConfig((prev) => {
      if (!prev) return prev;
      return {
        ...prev,
        rules: prev.rules.map((rule) => (rule.id === ruleId ? updater(rule) : rule)),
      };
    });
  };

  const normalizeRulePriorities = (rules: ClassificationRule[]) => {
    return rules
      .slice()
      .sort((a, b) => a.priority - b.priority)
      .map((rule, i) => ({ ...rule, priority: (i + 1) * 10 }));
  };

  const uniqueRuleId = (candidate: string, existing: ClassificationRule[]) => {
    const base = candidate
      .trim()
      .toLowerCase()
      .replace(/[^a-z0-9]+/g, "-")
      .replace(/^-+|-+$/g, "") || "custom-rule";
    const ids = new Set(existing.map((r) => r.id));
    if (!ids.has(base)) return base;
    let suffix = 2;
    while (ids.has(`${base}-${suffix}`)) suffix += 1;
    return `${base}-${suffix}`;
  };

  const tokenDraftKey = (ruleId: string, field: "extensions" | "keywords") => `${ruleId}:${field}`;

  const addRuleToken = (ruleId: string, field: "extensions" | "keywords") => {
    const draftKey = tokenDraftKey(ruleId, field);
    const raw = (ruleTokenDrafts[draftKey] || "").trim();
    if (!raw) return;
    const normalized = field === "extensions" ? raw.replace(/^\./, "").toLowerCase() : raw;

    updateRule(ruleId, (rule) => {
      const current = field === "extensions" ? rule.conditions.extensions : rule.conditions.filename_keywords;
      if (current.includes(normalized)) return rule;
      return {
        ...rule,
        conditions: {
          ...rule.conditions,
          [field]: [...current, normalized],
        },
      };
    });

    setRuleTokenDrafts((prev) => ({ ...prev, [draftKey]: "" }));
  };

  const removeRuleToken = (ruleId: string, field: "extensions" | "keywords", token: string) => {
    updateRule(ruleId, (rule) => ({
      ...rule,
      conditions: {
        ...rule.conditions,
        [field]: (field === "extensions" ? rule.conditions.extensions : rule.conditions.filename_keywords).filter((t) => t !== token),
      },
    }));
  };

  const addNewRule = () => {
    if (!ruleConfig) return;

    const name = newRuleDraft.name.trim();
    const destination = newRuleDraft.destination_folder.trim();
    if (!name || !destination) {
      setErrorMessage("New rule requires at least a rule name and destination folder.");
      return;
    }

    const parsedExtensions = newRuleDraft.extensions
      .split(",")
      .map((t) => t.trim().replace(/^\./, "").toLowerCase())
      .filter(Boolean);
    const parsedKeywords = newRuleDraft.keywords
      .split(",")
      .map((t) => t.trim())
      .filter(Boolean);

    const nextPriority =
      ruleConfig.rules.length === 0
        ? 10
        : Math.max(...ruleConfig.rules.map((r) => r.priority)) + 10;
    const idCandidate = newRuleDraft.id.trim() || name;
    const id = uniqueRuleId(idCandidate, ruleConfig.rules);

    const newRule: ClassificationRule = {
      id,
      name,
      category_path: newRuleDraft.category_path.trim() || "Custom",
      destination_folder: destination,
      priority: nextPriority,
      enabled: true,
      action: newRuleDraft.action,
      conditions: {
        extensions: parsedExtensions,
        filename_keywords: parsedKeywords,
        min_size_bytes: null,
        max_size_bytes: null,
      },
    };

    setRuleConfig((prev) => {
      if (!prev) return prev;
      return {
        ...prev,
        rules: normalizeRulePriorities([...prev.rules, newRule]),
      };
    });

    setNewRuleDraft({
      id: "",
      name: "",
      destination_folder: "",
      category_path: "",
      action: "move",
      extensions: "",
      keywords: "",
    });
    setStatusMessage(`Added new rule '${name}'. Save rules to persist.`);
    setErrorMessage(null);
  };

  const reorderRule = (ruleId: string, direction: "up" | "down") => {
    setRuleConfig((prev) => {
      if (!prev) return prev;
      const rules = [...prev.rules].sort((a, b) => a.priority - b.priority);
      const index = rules.findIndex((r) => r.id === ruleId);
      if (index < 0) return prev;

      const swapIndex = direction === "up" ? index - 1 : index + 1;
      if (swapIndex < 0 || swapIndex >= rules.length) return prev;

      const temp = rules[index];
      rules[index] = rules[swapIndex];
      rules[swapIndex] = temp;

      const normalized = normalizeRulePriorities(rules);
      return { ...prev, rules: normalized };
    });
  };

  const saveRuleConfig = async () => {
    if (!ruleConfig) return;
    setRulesSaving(true);
    setErrorMessage(null);
    try {
      const saved = await invoke<RuleConfig>("save_rule_config_cmd", { config: ruleConfig });
      const sortedRules = [...saved.rules].sort((a, b) => a.priority - b.priority);
      setRuleConfig({ ...saved, rules: sortedRules });
      setStatusMessage("Rule configuration saved successfully.");
    } catch (e) {
      console.error("Failed to save rule config", e);
      setErrorMessage("Failed to save rule configuration.");
    } finally {
      setRulesSaving(false);
    }
  };

  const previewRule = async (ruleId: string) => {
    if (!selectedPath) {
      setErrorMessage("Select a target folder before previewing rule matches.");
      return;
    }
    setPreviewLoadingRuleId(ruleId);
    setErrorMessage(null);
    try {
      const result = await invoke<string[]>("preview_rule_matches", {
        path: selectedPath,
        ruleId,
        maxResults: 25,
      });
      setRulePreview((prev) => ({ ...prev, [ruleId]: result }));
    } catch (e) {
      console.error("Failed preview rule", e);
      setErrorMessage("Failed to preview rule matches.");
    } finally {
      setPreviewLoadingRuleId(null);
    }
  };

  const handleUndo = async (manifest: TransactionManifest) => {
    setExecuting(true);
    setErrorMessage(null);
    try {
      await invoke("undo_moves", { manifest });
      setStatusMessage("Rollback completed.");
      await loadHistory();
      if (selectedPath) {
        await loadDirectory(selectedPath);
      }
    } catch (e) {
      console.error("Failed to undo moves", e);
      setErrorMessage("Rollback failed. Check permissions and try again.");
    } finally {
      setExecuting(false);
    }
  };

  const handleExecuteMoves = async () => {
    if (!selectedPath || files.length === 0) return;
    setExecuting(true);
    setErrorMessage(null);
    try {
      const manifest = await invoke<TransactionManifest>("execute_moves", { path: selectedPath, files });
      const successCount = manifest.moves.filter((m) => m.status === "success").length;
      const skippedCount = manifest.moves.filter((m) => m.status === "duplicate_skipped").length;
      const failedCount = manifest.moves.filter((m) => m.status === "failed").length;
      setStatusMessage(`Move complete: ${successCount} moved, ${skippedCount} duplicates skipped, ${failedCount} failed.`);
      // Reload directory post-move to update view
      await loadDirectory(selectedPath);
    } catch (e) {
      console.error("Move execution failed", e);
      setErrorMessage("Move execution failed. No further actions were applied automatically.");
    } finally {
      setExecuting(false);
    }
  };

  const categoryCounts = files.reduce((acc, f) => {
    if (f.is_dir) return acc;
    const cat = f.category || "Unknown";
    acc[cat] = (acc[cat] || 0) + 1;
    return acc;
  }, {} as Record<string, number>);

  const categoryChartData = Object.entries(categoryCounts)
    .map(([name, value]) => ({ name, value }))
    .sort((a, b) => b.value - a.value);

  const sizeCounts = files.reduce((acc, f) => {
    if (f.is_dir) return acc;
    const cat = f.category || "Unknown";
    // Convert bytes directly in the loop or after
    acc[cat] = (acc[cat] || 0) + f.size;
    return acc;
  }, {} as Record<string, number>);

  const sizeChartData = Object.entries(sizeCounts)
    .map(([name, value]) => ({ name, value: Math.round(value / (1024 * 1024)) }))
    .sort((a, b) => b.value - a.value);

  const aiClassifiedCount = files.filter(f => !f.is_dir && f.category && f.category !== "Unknown").length;
  const safeToMoveCount = files.filter(f => !f.is_dir && f.suggested_folder).length;
  const totalFiles = files.filter(f => !f.is_dir).length;
  const updateProgressPercent =
    updateStatus.totalBytes && updateStatus.totalBytes > 0
      ? Math.min(100, Math.round(((updateStatus.downloadedBytes ?? 0) / updateStatus.totalBytes) * 100))
      : null;
  const isUpdateBusy =
    updateStatus.phase === "checking" ||
    updateStatus.phase === "downloading" ||
    updateStatus.phase === "installing";

  return (
    <div className="flex h-screen w-full bg-background text-foreground overflow-hidden">
      <div className="pointer-events-none absolute inset-x-0 top-0 h-32 bg-gradient-to-b from-primary/5 to-transparent" />

      {/* Sidebar Navigation */}
      <div className="w-64 border-r border-border bg-card/95 backdrop-blur flex flex-col shrink-0">
        <div className="p-4 flex items-center gap-2 border-b border-border">
          <Folder className="h-6 w-6 text-primary" />
          <h1 className="font-bold text-lg">FileSorter AI</h1>
        </div>

        <div className="flex-1 py-4 flex flex-col gap-2 px-2">
          <Button
            variant={currentView === "dashboard" ? "secondary" : "ghost"}
            className="justify-start gap-2"
            onClick={() => setCurrentView("dashboard")}
          >
            <FileSearch size={18} />
            Dashboard
          </Button>
          <Button
            variant={currentView === "rules" ? "secondary" : "ghost"}
            className="justify-start gap-2"
            onClick={() => {
              setCurrentView("rules");
              loadRuleConfig();
            }}
          >
            <Settings size={18} />
            Rules & Config
          </Button>
          <Button
            variant={currentView === "history" ? "secondary" : "ghost"}
            className="justify-start gap-2"
            onClick={() => {
              setCurrentView("history");
              loadHistory();
            }}
          >
            <History size={18} />
            Action History
          </Button>
        </div>
      </div>

      {/* Main Content Area */}
      <div className="flex-1 flex flex-col h-full bg-background/50">

        {/* Header */}
        <header className="h-16 border-b border-border flex items-center px-6 justify-between shrink-0">
          <h2 className="text-lg font-semibold tracking-tight">
            {currentView === "dashboard" ? "System Scan" : currentView === "history" ? "Action History" : "Rules & Config"}
          </h2>
          <div className="flex items-center gap-2">
            <Button
              variant="outline"
              size="icon"
              onClick={toggleThemeMode}
              title={themeMode === "dark" ? "Switch to light theme" : "Switch to dark theme"}
              aria-label={themeMode === "dark" ? "Switch to light theme" : "Switch to dark theme"}
            >
              {themeMode === "dark" ? <Sun size={16} /> : <Moon size={16} />}
            </Button>
            {updateStatus.phase === "ready-to-install" ? (
              <Button size="sm" className="gap-2" onClick={installDownloadedUpdate}>
                <Download size={16} />
                Install Update
              </Button>
            ) : (
              <Button
                variant="outline"
                size="sm"
                className="gap-2"
                onClick={() => void autoCheckAndDownloadUpdate()}
                disabled={isUpdateBusy}
              >
                <RefreshCw size={16} className={isUpdateBusy ? "animate-spin" : ""} />
                {updateStatus.phase === "checking"
                  ? "Checking..."
                  : updateStatus.phase === "downloading"
                    ? updateProgressPercent !== null
                      ? `Downloading ${updateProgressPercent}%`
                      : "Downloading..."
                    : updateStatus.phase === "installing"
                      ? "Installing..."
                      : "Check Updates"}
              </Button>
            )}
            <Button
              variant="outline"
              size="sm"
              className="gap-2"
              onClick={() => setAISettingsDialogOpen(true)}
              disabled={aiLoading}
            >
              <span
                className={`h-2.5 w-2.5 rounded-full ${aiAvailable ? "bg-green-500" : "bg-red-500"}`}
                aria-hidden="true"
              />
              {aiAvailable ? "AI Available" : "AI Unavailable"}
            </Button>
            {(currentView === "dashboard" || currentView === "rules") && (
              <>
              <Button variant="outline" size="sm" onClick={handleSelectFolder}>
                Choose Target Folder
              </Button>
              {currentView === "dashboard" ? (
                <Button size="sm" className="gap-2" disabled={!selectedPath || files.length === 0 || executing} onClick={handleExecuteMoves}>
                  <Play size={16} />
                  {executing ? "Organizing..." : "Execute Full Move"}
                </Button>
              ) : (
                <>
                  <Button variant="outline" size="sm" disabled={rulesLoading} onClick={loadRuleConfig}>
                    Reload Rules
                  </Button>
                  <Button size="sm" disabled={!ruleConfig || rulesSaving} onClick={saveRuleConfig}>
                    {rulesSaving ? "Saving..." : "Save Rules"}
                  </Button>
                </>
              )}
              </>
            )}
          </div>
        </header>

        <Dialog open={aiSettingsDialogOpen} onOpenChange={setAISettingsDialogOpen}>
          <DialogContent className="sm:max-w-lg">
            <DialogHeader>
              <DialogTitle>AI Provider Settings</DialogTitle>
              <DialogDescription>
                Configure provider, API key, and model. Green means the selected provider validated successfully.
              </DialogDescription>
            </DialogHeader>

            <div className="space-y-4">
              <div className="flex items-center justify-between rounded-md border border-border bg-muted/20 px-3 py-2">
                <div className="text-sm">
                  <p className="font-medium">Provider status</p>
                  <p className="text-xs text-muted-foreground">{aiStatusMessage}</p>
                </div>
                <span
                  className={`h-3 w-3 rounded-full ${aiAvailable ? "bg-green-500" : "bg-red-500"}`}
                  aria-hidden="true"
                />
              </div>

              <div className="space-y-2">
                <Label htmlFor="ai-enabled">Enable AI sorting</Label>
                <label className="inline-flex items-center gap-2 text-sm">
                  <input
                    id="ai-enabled"
                    type="checkbox"
                    checked={aiSettingsDraft.enabled}
                    onChange={(e) =>
                      setAISettingsDraft((prev) => ({
                        ...prev,
                        enabled: e.target.checked,
                      }))
                    }
                  />
                  Enable AI sorting
                </label>
              </div>

              <div className="space-y-2">
                <Label htmlFor="ai-first-fallback">AI-first sorting with normal sorting fallback</Label>
                <label className="inline-flex items-center gap-2 text-sm">
                  <input
                    id="ai-first-fallback"
                    type="checkbox"
                    checked={aiSettingsDraft.ai_first_with_fallback}
                    disabled={!aiSettingsDraft.enabled || aiSettingsDraft.complete_ai_sorting}
                    onChange={(e) =>
                      setAISettingsDraft((prev) => ({
                        ...prev,
                        ai_first_with_fallback: e.target.checked,
                      }))
                    }
                  />
                  Run AI before rules; if AI cannot classify a file, apply normal rule sorting
                </label>
              </div>

              <div className="space-y-2">
                <Label htmlFor="ai-complete-sorting">Complete AI sorting</Label>
                <label className="inline-flex items-center gap-2 text-sm">
                  <input
                    id="ai-complete-sorting"
                    type="checkbox"
                    checked={aiSettingsDraft.complete_ai_sorting}
                    disabled={!aiSettingsDraft.enabled}
                    onChange={(e) =>
                      setAISettingsDraft((prev) => ({
                        ...prev,
                        complete_ai_sorting: e.target.checked,
                      }))
                    }
                  />
                  Let AI decide folders and file placement with no rule fallback
                </label>
              </div>

              <div className="space-y-2">
                <Label htmlFor="provider-select">Model Provider</Label>
                <select
                  id="provider-select"
                  className="h-10 w-full rounded-md border border-input bg-background px-3 text-sm"
                  value={aiSettingsDraft.selected_provider}
                  onChange={(e) => handleProviderChange(e.target.value as AIProvider)}
                >
                  {PROVIDER_OPTIONS.map((option) => (
                    <option key={option.value} value={option.value}>
                      {option.label}
                    </option>
                  ))}
                </select>
              </div>

              <div className="space-y-2">
                <Label htmlFor="api-key-input">API Key</Label>
                <Input
                  id="api-key-input"
                  type="password"
                  autoComplete="off"
                  placeholder={aiSettingsEnvelope?.api_key_present ? "Stored in credential vault (enter to replace)" : "Enter API key"}
                  value={aiApiKeyDraft}
                  onChange={(e) => setAIApiKeyDraft(e.target.value)}
                />
              </div>

              {providerNeedsBaseUrl(aiSettingsDraft.selected_provider) && (
                <div className="space-y-2">
                  <Label htmlFor="base-url-input">Base URL</Label>
                  <Input
                    id="base-url-input"
                    placeholder={
                      aiSettingsDraft.selected_provider === "ollama"
                        ? "http://localhost:11434"
                        : "https://openrouter.ai/api/v1"
                    }
                    value={aiSettingsDraft.custom_base_url ?? ""}
                    onChange={(e) =>
                      setAISettingsDraft((prev) => ({
                        ...prev,
                        custom_base_url: e.target.value || null,
                      }))
                    }
                  />
                </div>
              )}

              <div className="space-y-2">
                <div className="flex items-center justify-between">
                  <Label htmlFor="model-name-select">Model Name</Label>
                  <Button variant="outline" size="sm" onClick={validateAndLoadModels} disabled={aiModelsLoading}>
                    {aiModelsLoading ? "Loading..." : "Validate & Load Models"}
                  </Button>
                </div>
                <select
                  id="model-name-select"
                  className="h-10 w-full rounded-md border border-input bg-background px-3 text-sm"
                  value={aiSettingsDraft.selected_model}
                  onChange={(e) =>
                    setAISettingsDraft((prev) => ({
                      ...prev,
                      selected_model: e.target.value,
                    }))
                  }
                >
                  {aiModels.length === 0 ? (
                    <option value={aiSettingsDraft.selected_model}>{aiSettingsDraft.selected_model}</option>
                  ) : (
                    aiModels.map((model) => (
                      <option key={model} value={model}>
                        {model}
                      </option>
                    ))
                  )}
                </select>
              </div>
            </div>

            <DialogFooter>
              <Button variant="outline" onClick={() => setAISettingsDialogOpen(false)}>
                Close
              </Button>
              <Button onClick={saveAISettings} disabled={aiSaving}>
                {aiSaving ? "Saving..." : "Save Settings"}
              </Button>
            </DialogFooter>
          </DialogContent>
        </Dialog>

        {/* Dynamic Content */}
        <ScrollArea className="flex-1 p-6">
          <div className="flex flex-col gap-6 max-w-5xl mx-auto">
            {errorMessage && (
              <div className="rounded-md border border-destructive/40 bg-destructive/10 p-3 text-sm text-destructive">
                {errorMessage}
              </div>
            )}
            {statusMessage && !errorMessage && (
              <div className="rounded-md border border-primary/30 bg-primary/10 p-3 text-sm text-primary">
                {statusMessage}
              </div>
            )}
            {updateStatus.phase !== "idle" && (
              <div
                className={`rounded-md border p-3 text-sm ${
                  updateStatus.phase === "error"
                    ? "border-destructive/40 bg-destructive/10 text-destructive"
                    : updateStatus.phase === "ready-to-install" || updateStatus.phase === "installed"
                      ? "border-warning-border bg-warning-muted text-warning-muted-foreground"
                      : "border-border bg-muted/30 text-muted-foreground"
                }`}
              >
                <div className="flex items-center justify-between gap-3">
                  <span>{updateStatus.message}</span>
                  {updateStatus.version && (
                    <span className="text-xs font-medium rounded-full border border-border px-2 py-0.5">
                      v{updateStatus.version}
                    </span>
                  )}
                </div>
              </div>
            )}
            {currentView === "history" ? (
              <div className="flex flex-col gap-4">
                {history.length === 0 ? (
                  <div className="text-center p-8 text-muted-foreground border rounded-md">
                    No action history found.
                  </div>
                ) : (
                  history.map((manifest, idx) => (
                    <Card key={idx} className="border-border">
                      <CardHeader className="py-3 border-b border-border bg-muted/20 flex flex-row items-center justify-between">
                        <div>
                          <CardTitle className="text-sm">Txn: {manifest.transaction_id.split('_')[1]}</CardTitle>
                          <CardDescription className="text-xs">
                            {manifest.root_folder} | {new Date(manifest.timestamp).toLocaleString()}
                          </CardDescription>
                        </div>
                        <Button variant="destructive" size="sm" onClick={() => handleUndo(manifest)} disabled={executing}>
                          Rollback
                        </Button>
                      </CardHeader>
                      <CardContent className="pt-4">
                        <div className="flex flex-col gap-2 max-h-[200px] overflow-auto">
                          {manifest.moves.map((m: MoveOperation, mIdx: number) => (
                            <div key={mIdx} className="text-xs flex gap-2 w-full truncate">
                              <span className={m.status === 'success' ? 'text-green-500' : m.status === 'rolled_back' ? 'text-orange-500' : 'text-red-500'}>
                                [{m.status.toUpperCase()}]
                              </span>
                              <span className="truncate flex-1 text-muted-foreground" title={m.target_path}>
                                {m.target_path.split('\\').pop()} {"<-"} {m.original_path.split('\\').pop()}
                              </span>
                            </div>
                          ))}
                        </div>
                      </CardContent>
                    </Card>
                  ))
                )}
              </div>
            ) : currentView === "rules" ? (
              <div className="flex flex-col gap-4">
                <Card>
                  <CardHeader>
                    <CardTitle>Rule Configuration</CardTitle>
                    <CardDescription>
                      First-match-wins policy by priority order. Move rules up/down to resolve conflicts.
                    </CardDescription>
                  </CardHeader>
                  <CardContent className="space-y-3">
                    <div className="grid grid-cols-1 md:grid-cols-2 gap-3">
                      <div>
                        <p className="text-xs text-muted-foreground mb-1">Unknown Folder</p>
                        <Input
                          value={ruleConfig?.unknown_folder ?? ""}
                          onChange={(e) =>
                            setRuleConfig((prev) =>
                              prev
                                ? {
                                    ...prev,
                                    unknown_folder: e.target.value,
                                  }
                                : prev
                            )
                          }
                          disabled={!ruleConfig}
                        />
                      </div>
                      <div>
                        <p className="text-xs text-muted-foreground mb-1">Selected Preview Folder</p>
                        <div className="h-9 rounded-md border border-input px-3 text-xs flex items-center text-muted-foreground truncate">
                          {selectedPath || "No folder selected"}
                        </div>
                      </div>
                    </div>
                  </CardContent>
                </Card>

                <Card>
                  <CardHeader>
                    <CardTitle>Rules</CardTitle>
                    <CardDescription>
                      Enable/disable, edit destination/category, reorder by priority, then save.
                    </CardDescription>
                  </CardHeader>
                  <CardContent>
                    {rulesLoading ? (
                      <div className="text-sm text-muted-foreground">Loading rule configuration...</div>
                    ) : !ruleConfig ? (
                      <div className="text-sm text-muted-foreground">No rule configuration loaded.</div>
                    ) : (
                      <Table>
                        <TableHeader>
                          <TableRow>
                            <TableHead className="w-16">On</TableHead>
                            <TableHead className="w-20">Order</TableHead>
                            <TableHead>Rule</TableHead>
                            <TableHead>Destination</TableHead>
                            <TableHead className="w-[360px]">Extensions / Keywords</TableHead>
                            <TableHead className="w-32">Actions</TableHead>
                          </TableRow>
                        </TableHeader>
                        <TableBody>
                          <TableRow>
                            <TableCell colSpan={6}>
                              <div className="grid grid-cols-1 md:grid-cols-6 gap-2 items-center">
                                <Input
                                  value={newRuleDraft.name}
                                  onChange={(e) => setNewRuleDraft((prev) => ({ ...prev, name: e.target.value }))}
                                  placeholder="New rule name"
                                  className="md:col-span-1"
                                />
                                <Input
                                  value={newRuleDraft.destination_folder}
                                  onChange={(e) => setNewRuleDraft((prev) => ({ ...prev, destination_folder: e.target.value }))}
                                  placeholder="Destination folder"
                                  className="md:col-span-1"
                                />
                                <Input
                                  value={newRuleDraft.category_path}
                                  onChange={(e) => setNewRuleDraft((prev) => ({ ...prev, category_path: e.target.value }))}
                                  placeholder="Category path"
                                  className="md:col-span-1"
                                />
                                <Input
                                  value={newRuleDraft.extensions}
                                  onChange={(e) => setNewRuleDraft((prev) => ({ ...prev, extensions: e.target.value }))}
                                  placeholder="Extensions csv (jpg,png,tar.gz)"
                                  className="md:col-span-1"
                                />
                                <Input
                                  value={newRuleDraft.keywords}
                                  onChange={(e) => setNewRuleDraft((prev) => ({ ...prev, keywords: e.target.value }))}
                                  placeholder="Keywords csv (invoice,report)"
                                  className="md:col-span-1"
                                />
                                <Button className="md:col-span-1" onClick={addNewRule}>
                                  <Plus size={14} />
                                  Add Rule
                                </Button>
                              </div>
                            </TableCell>
                          </TableRow>
                          {ruleConfig.rules
                            .slice()
                            .sort((a, b) => a.priority - b.priority)
                            .map((rule, idx) => (
                              <Fragment key={rule.id}>
                                <TableRow key={rule.id}>
                                  <TableCell>
                                    <input
                                      type="checkbox"
                                      checked={rule.enabled}
                                      onChange={(e) =>
                                        updateRule(rule.id, (r) => ({
                                          ...r,
                                          enabled: e.target.checked,
                                        }))
                                      }
                                    />
                                  </TableCell>
                                  <TableCell className="text-xs">{rule.priority}</TableCell>
                                  <TableCell>
                                    <div className="space-y-1">
                                      <Input
                                        value={rule.name}
                                        onChange={(e) =>
                                          updateRule(rule.id, (r) => ({ ...r, name: e.target.value }))
                                        }
                                      />
                                      <p className="text-[11px] text-muted-foreground">{rule.id}</p>
                                    </div>
                                  </TableCell>
                                  <TableCell>
                                    <div className="space-y-1">
                                      <Input
                                        value={rule.destination_folder}
                                        onChange={(e) =>
                                          updateRule(rule.id, (r) => ({ ...r, destination_folder: e.target.value }))
                                        }
                                      />
                                      <Input
                                        value={rule.category_path}
                                        onChange={(e) =>
                                          updateRule(rule.id, (r) => ({ ...r, category_path: e.target.value }))
                                        }
                                        placeholder="Category path"
                                      />
                                    </div>
                                  </TableCell>
                                  <TableCell>
                                    <div className="space-y-2">
                                      <div className="flex flex-wrap gap-1">
                                        {rule.conditions.extensions.map((ext) => (
                                          <span key={`${rule.id}-ext-${ext}`} className="inline-flex items-center gap-1 rounded-full border border-border px-2 py-0.5 text-[11px]">
                                            .{ext}
                                            <button
                                              className="text-muted-foreground hover:text-foreground"
                                              onClick={() => removeRuleToken(rule.id, "extensions", ext)}
                                              aria-label={`Remove extension ${ext}`}
                                            >
                                              <X size={12} />
                                            </button>
                                          </span>
                                        ))}
                                      </div>
                                      <Input
                                        value={ruleTokenDrafts[`${rule.id}:extensions`] ?? ""}
                                        placeholder="Add extension and press Enter"
                                        onChange={(e) =>
                                          setRuleTokenDrafts((prev) => ({ ...prev, [`${rule.id}:extensions`]: e.target.value }))
                                        }
                                        onKeyDown={(e) => {
                                          if (e.key === "Enter") {
                                            e.preventDefault();
                                            addRuleToken(rule.id, "extensions");
                                          }
                                        }}
                                        onBlur={() => addRuleToken(rule.id, "extensions")}
                                      />

                                      <div className="flex flex-wrap gap-1">
                                        {rule.conditions.filename_keywords.map((kw) => (
                                          <span key={`${rule.id}-kw-${kw}`} className="inline-flex items-center gap-1 rounded-full border border-border px-2 py-0.5 text-[11px]">
                                            {kw}
                                            <button
                                              className="text-muted-foreground hover:text-foreground"
                                              onClick={() => removeRuleToken(rule.id, "keywords", kw)}
                                              aria-label={`Remove keyword ${kw}`}
                                            >
                                              <X size={12} />
                                            </button>
                                          </span>
                                        ))}
                                      </div>
                                      <Input
                                        value={ruleTokenDrafts[`${rule.id}:keywords`] ?? ""}
                                        placeholder="Add keyword and press Enter"
                                        onChange={(e) =>
                                          setRuleTokenDrafts((prev) => ({ ...prev, [`${rule.id}:keywords`]: e.target.value }))
                                        }
                                        onKeyDown={(e) => {
                                          if (e.key === "Enter") {
                                            e.preventDefault();
                                            addRuleToken(rule.id, "keywords");
                                          }
                                        }}
                                        onBlur={() => addRuleToken(rule.id, "keywords")}
                                      />
                                    </div>
                                  </TableCell>
                                  <TableCell>
                                    <div className="flex items-center gap-1">
                                      <Button
                                        variant="ghost"
                                        size="icon"
                                        onClick={() => reorderRule(rule.id, "up")}
                                        disabled={idx === 0}
                                      >
                                        <ArrowUp size={14} />
                                      </Button>
                                      <Button
                                        variant="ghost"
                                        size="icon"
                                        onClick={() => reorderRule(rule.id, "down")}
                                        disabled={idx === ruleConfig.rules.length - 1}
                                      >
                                        <ArrowDown size={14} />
                                      </Button>
                                      <Button
                                        variant="ghost"
                                        size="icon"
                                        onClick={() => previewRule(rule.id)}
                                        disabled={!selectedPath || previewLoadingRuleId === rule.id}
                                        title="Preview matched files"
                                      >
                                        <Eye size={14} />
                                      </Button>
                                    </div>
                                  </TableCell>
                                </TableRow>
                                {rulePreview[rule.id] && (
                                  <TableRow key={`${rule.id}-preview`}>
                                    <TableCell colSpan={6}>
                                      <div className="text-xs text-muted-foreground space-y-1">
                                        <p className="font-medium text-foreground">Preview ({rulePreview[rule.id].length} files)</p>
                                        {rulePreview[rule.id].length === 0 ? (
                                          <p>No files matched this rule in the selected folder.</p>
                                        ) : (
                                          rulePreview[rule.id].map((path, previewIdx) => (
                                            <p key={`${rule.id}-preview-${previewIdx}`} className="truncate" title={path}>{path}</p>
                                          ))
                                        )}
                                      </div>
                                    </TableCell>
                                  </TableRow>
                                )}
                              </Fragment>
                            ))}
                        </TableBody>
                      </Table>
                    )}
                  </CardContent>
                </Card>
              </div>
            ) : (
              <>
                {/* Target Information Card */}
                <Card>
                  <CardHeader>
                    <CardTitle>Target Directory</CardTitle>
                    <CardDescription>Select a folder to begin organizing.</CardDescription>
                  </CardHeader>
                  <CardContent>
                    <div className="flex items-center justify-between bg-muted/50 p-3 rounded-md border border-border">
                      <span className="font-mono text-sm text-muted-foreground truncate">
                        {selectedPath || "No folder selected..."}
                      </span>
                      {selectedPath && (
                        <span className="text-xs bg-primary/20 text-primary px-2 py-1 rounded-full font-medium">Ready</span>
                      )}
                    </div>
                  </CardContent>
                </Card>

                {/* Analysis Stats */}
                <div className="grid grid-cols-1 md:grid-cols-3 gap-4">
                  <Card>
                    <CardHeader className="pb-2">
                      <CardTitle className="text-sm font-medium text-muted-foreground">Files Discovered</CardTitle>
                    </CardHeader>
                    <CardContent>
                      <div className="text-2xl font-bold">{scanning ? "..." : totalFiles}</div>
                    </CardContent>
                  </Card>

                  <Card>
                    <CardHeader className="pb-2">
                      <CardTitle className="text-sm font-medium text-muted-foreground">AI Classifications</CardTitle>
                    </CardHeader>
                    <CardContent>
                      <div className="text-2xl font-bold text-blue-500">{scanning ? "..." : aiClassifiedCount}</div>
                    </CardContent>
                  </Card>

                  <Card>
                    <CardHeader className="pb-2">
                      <CardTitle className="text-sm font-medium text-muted-foreground">Safe to Move</CardTitle>
                    </CardHeader>
                    <CardContent>
                      <div className="text-2xl font-bold text-green-500">{scanning ? "..." : safeToMoveCount}</div>
                    </CardContent>
                  </Card>
                </div>

                {/* Visual Analytics */}
                {files.length > 0 && (
                  <Suspense
                    fallback={
                      <Card>
                        <CardContent className="py-8 text-sm text-muted-foreground">
                          Loading analytics charts...
                        </CardContent>
                      </Card>
                    }
                  >
                    <AnalyticsCharts categoryChartData={categoryChartData} sizeChartData={sizeChartData} />
                  </Suspense>
                )}

                <Separator className="my-4" />

                {/* Preview Tree (Grouped Folder View) */}
                {files.length > 0 && (
                  <Card className="flex flex-col h-[600px] border-primary/20">
                    <CardHeader className="py-3 border-b border-border bg-muted/20">
                      <CardTitle className="text-sm">Action Manifest (Dry-Run Preview)</CardTitle>
                      <CardDescription className="text-xs">
                        This is a preview of how your files will be organized.
                      </CardDescription>
                    </CardHeader>
                    <ScrollArea className="flex-1">
                      <div className="p-4 flex flex-col gap-4">
                        {/* Convert flat files into grouped structure by suggested_folder */}
                        {Object.entries(
                          files
                            .filter(f => !f.is_dir) // Only show files we are moving
                            .reduce((acc, file) => {
                              const folder = file.suggested_folder || "Uncategorized";
                              if (!acc[folder]) acc[folder] = [];
                              acc[folder].push(file);
                              return acc;
                            }, {} as Record<string, FileNode[]>)
                        ).sort(([a], [b]) => a.localeCompare(b)).map(([folderName, groupedFiles]) => (
                          <div key={folderName} className="border border-border rounded-md overflow-hidden">
                            <div className="bg-secondary p-2 px-3 flex items-center gap-2 border-b border-border">
                              <FolderIcon size={16} className="text-primary" />
                              <span className="font-semibold text-sm">{folderName}</span>
                              <span className="text-xs text-muted-foreground ml-auto bg-background px-2 py-0.5 rounded-full">
                                {groupedFiles.length} items
                              </span>
                            </div>
                            <div className="bg-background flex flex-col divide-y divide-border">
                              {groupedFiles.map((f, i) => (
                                <div key={i} className="flex items-center gap-3 p-2 px-4 hover:bg-muted/30 text-xs text-muted-foreground">
                                  <FileIcon size={14} className="shrink-0" />
                                  <span className="truncate flex-1 font-mono">{f.name}</span>
                                  <span className="shrink-0 w-24 text-right">{(f.size / 1024).toFixed(1)} KB</span>
                                  <span className="shrink-0 w-24 text-right text-primary/80">{f.category || "Unknown"}</span>
                                </div>
                              ))}
                            </div>
                          </div>
                        ))}

                        {files.filter(f => !f.is_dir).length === 0 && (
                          <div className="text-center p-8 text-muted-foreground text-sm">
                            No loose files found to organize.
                          </div>
                        )}
                      </div>
                    </ScrollArea>
                  </Card>
                )}

                {/* Safety Notice */}
                <div className="bg-warning-muted border border-warning-border rounded-lg p-4 flex gap-4 items-start">
                  <ShieldAlert className="text-warning mt-1 shrink-0" />
                  <div>
                    <h4 className="font-medium text-warning">Dry-Run Enabled by Default</h4>
                    <p className="text-sm text-warning-muted-foreground mt-1">
                      All scans simulate operations first. No files are moved until you explicitly review and approve the Action Manifest.
                    </p>
                  </div>
                </div>
              </>
            )}
          </div>
        </ScrollArea>
      </div>
    </div>
  );
}
