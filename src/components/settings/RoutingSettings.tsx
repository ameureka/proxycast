import { useState } from "react";
import { Route, Shield, AlertTriangle } from "lucide-react";
import { cn } from "@/lib/utils";
import { RoutingPage } from "../routing/RoutingPage";
import { ResiliencePage } from "../resilience/ResiliencePage";

type Tab = "routing" | "resilience";

const tabs = [
  { id: "routing" as Tab, label: "智能路由", icon: Route },
  { id: "resilience" as Tab, label: "容错配置", icon: Shield },
];

export function RoutingSettings() {
  const [activeTab, setActiveTab] = useState<Tab>("routing");

  return (
    <div className="space-y-6">
      <div>
        <div className="flex items-center gap-2">
          <h3 className="text-lg font-semibold">路由管理</h3>
          <div className="flex items-center gap-1 px-2 py-1 bg-yellow-100 text-yellow-800 text-xs rounded-md">
            <AlertTriangle className="h-3 w-3" />
            实验功能
          </div>
        </div>
        <p className="text-muted-foreground">
          配置智能路由规则和容错策略（实验性功能，可能不稳定）
        </p>
      </div>

      {/* Tab 切换 */}
      <div className="flex gap-1 border-b">
        {tabs.map((tab) => (
          <button
            key={tab.id}
            onClick={() => setActiveTab(tab.id)}
            className={cn(
              "flex items-center gap-2 px-4 py-2 text-sm font-medium border-b-2 -mb-px transition-colors",
              activeTab === tab.id
                ? "border-primary text-primary"
                : "border-transparent text-muted-foreground hover:text-foreground",
            )}
          >
            <tab.icon className="h-4 w-4" />
            {tab.label}
          </button>
        ))}
      </div>

      {/* Tab 内容 */}
      <div className="pt-2">
        {activeTab === "routing" && <RoutingPageContent />}
        {activeTab === "resilience" && <ResiliencePageContent />}
      </div>
    </div>
  );
}

// 路由页面内容（去掉标题）
function RoutingPageContent() {
  return (
    <div className="routing-content">
      <RoutingPage hideHeader />
    </div>
  );
}

// 容错页面内容（去掉标题）
function ResiliencePageContent() {
  return (
    <div className="resilience-content">
      <ResiliencePage hideHeader />
    </div>
  );
}
