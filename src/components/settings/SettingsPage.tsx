import { useState } from "react";
import { cn } from "@/lib/utils";
import { GeneralSettings } from "./GeneralSettings";
import { ProxySettings } from "./ProxySettings";
import { DirectorySettings } from "./DirectorySettings";
import { AboutSection } from "./AboutSection";
import { TlsSettings } from "./TlsSettings";
import { QuotaSettings } from "./QuotaSettings";
import { RemoteManagementSettings } from "./RemoteManagementSettings";

type SettingsTab = "general" | "proxy" | "security" | "advanced" | "about";

const tabs: { id: SettingsTab; label: string }[] = [
  { id: "general", label: "通用" },
  { id: "proxy", label: "代理服务" },
  { id: "security", label: "安全" },
  { id: "advanced", label: "高级" },
  { id: "about", label: "关于" },
];

export function SettingsPage() {
  const [activeTab, setActiveTab] = useState<SettingsTab>("general");

  return (
    <div className="h-full flex flex-col">
      <div className="mb-6">
        <h2 className="text-2xl font-bold">设置</h2>
        <p className="text-muted-foreground">配置应用参数和偏好</p>
      </div>

      {/* 标签页 */}
      <div className="flex gap-1 border-b mb-6">
        {tabs.map((tab) => (
          <button
            key={tab.id}
            onClick={() => setActiveTab(tab.id)}
            className={cn(
              "px-4 py-2 text-sm font-medium transition-colors relative",
              activeTab === tab.id
                ? "text-primary"
                : "text-muted-foreground hover:text-foreground",
            )}
          >
            {tab.label}
            {activeTab === tab.id && (
              <div className="absolute bottom-0 left-0 right-0 h-0.5 bg-primary" />
            )}
          </button>
        ))}
      </div>

      {/* 内容区域 */}
      <div className="flex-1 overflow-auto">
        {activeTab === "general" && <GeneralSettings />}
        {activeTab === "proxy" && <ProxySettings />}
        {activeTab === "security" && (
          <div className="space-y-6 max-w-2xl">
            <TlsSettings />
            <RemoteManagementSettings />
          </div>
        )}
        {activeTab === "advanced" && (
          <div className="space-y-6 max-w-2xl">
            <DirectorySettings />
            <QuotaSettings />
          </div>
        )}
        {activeTab === "about" && <AboutSection />}
      </div>
    </div>
  );
}
