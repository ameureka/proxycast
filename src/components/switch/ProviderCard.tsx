import { Check, Edit2, Trash2, Zap } from "lucide-react";
import { Provider } from "@/lib/api/switch";
import { cn } from "@/lib/utils";
import { ProviderIcon } from "@/icons/providers";

// 从供应商名称和分类推断图标类型
function getProviderTypeFromName(name: string, category: string): string {
  const lowerName = name.toLowerCase();

  // 精确匹配
  if (lowerName.includes("智谱") || lowerName.includes("glm")) return "zhipu";
  if (lowerName.includes("proxycast")) return "proxycast";
  if (lowerName.includes("deepseek")) return "deepseek";
  if (lowerName.includes("kimi")) return "kimi";
  if (lowerName.includes("minimax")) return "minimax";
  if (lowerName.includes("doubao") || lowerName.includes("豆包"))
    return "doubao";
  if (lowerName.includes("qwen") || lowerName.includes("通义")) return "qwen";
  if (lowerName.includes("claude")) return "claude";
  if (lowerName.includes("anthropic")) return "anthropic";
  if (lowerName.includes("openai")) return "openai";
  if (lowerName.includes("gemini")) return "gemini";
  if (lowerName.includes("google")) return "google";
  if (lowerName.includes("kiro")) return "kiro";
  if (lowerName.includes("azure")) return "azure";
  if (lowerName.includes("alibaba") || lowerName.includes("阿里"))
    return "alibaba";
  if (lowerName.includes("copilot")) return "copilot";

  // 根据分类推断
  if (category === "custom") return "custom";
  if (category === "proxy") return "amp";

  // 默认回退
  return lowerName.replace(/\s+/g, "");
}

interface ProviderCardProps {
  provider: Provider;
  isCurrent: boolean;
  onSwitch: () => void;
  onEdit: () => void;
  onDelete: () => void;
}

export function ProviderCard({
  provider,
  isCurrent,
  onSwitch,
  onEdit,
  onDelete,
}: ProviderCardProps) {
  return (
    <div
      className={cn(
        "relative rounded-lg border p-4 transition-all",
        isCurrent
          ? "border-primary bg-primary/5 ring-1 ring-primary"
          : "hover:border-muted-foreground/50",
      )}
    >
      {isCurrent && (
        <div className="absolute -top-2 -right-2 rounded-full bg-primary p-1">
          <Check className="h-3 w-3 text-primary-foreground" />
        </div>
      )}

      <div className="flex items-start justify-between">
        <div className="flex items-center gap-2">
          <div className="h-8 w-8 rounded-lg flex items-center justify-center">
            <ProviderIcon
              providerType={getProviderTypeFromName(
                provider.name,
                provider.category || "",
              )}
              size={24}
              showFallback={true}
            />
          </div>
          <div>
            <h3 className="font-medium">{provider.name}</h3>
            {provider.category && (
              <span className="text-xs text-muted-foreground">
                {provider.category}
              </span>
            )}
          </div>
        </div>

        <div className="flex gap-1">
          <button
            onClick={onEdit}
            className="p-1.5 rounded hover:bg-muted"
            title="编辑"
          >
            <Edit2 className="h-3.5 w-3.5" />
          </button>
          <button
            onClick={(e) => {
              e.stopPropagation();
              if (!isCurrent) {
                onDelete();
              }
            }}
            disabled={isCurrent}
            className={cn(
              "p-1.5 rounded text-destructive",
              isCurrent
                ? "opacity-30 cursor-not-allowed"
                : "hover:bg-destructive/10",
            )}
            title={isCurrent ? "无法删除当前使用中的配置" : "删除"}
          >
            <Trash2 className="h-3.5 w-3.5" />
          </button>
        </div>
      </div>

      {provider.notes && (
        <p className="mt-2 text-sm text-muted-foreground line-clamp-2">
          {provider.notes}
        </p>
      )}

      <div className="mt-4">
        {isCurrent ? (
          <span className="text-sm text-primary font-medium">当前使用中</span>
        ) : (
          <button
            onClick={onSwitch}
            className="flex items-center gap-1.5 text-sm text-muted-foreground hover:text-foreground"
          >
            <Zap className="h-3.5 w-3.5" />
            切换到此配置
          </button>
        )}
      </div>
    </div>
  );
}
