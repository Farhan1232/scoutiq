export type NavId =
  | "dashboard"
  | "squad"
  | "transfers"
  | "tactics"
  | "analytics"
  | "finances"
  | "scouting"
  | "saveAnalyse"
  | "settings";

export type TrendDirection = "up" | "down" | "flat";

export interface NavItem {
  id: NavId;
  icon: IconName;
  badge?: string;
}

export type IconName =
  | "dashboard"
  | "squad"
  | "transfers"
  | "tactics"
  | "analytics"
  | "finances"
  | "scouting"
  | "settings"
  | "search"
  | "bell"
  | "chevronDown"
  | "sparkles"
  | "arrowUpRight"
  | "arrowDownRight"
  | "minus"
  | "target"
  | "calendar"
  | "command"
  | "sun"
  | "moon"
  | "monitor"
  | "google"
  | "apple"
  | "mail"
  | "user"
  | "logOut"
  | "upload"
  | "cloud"
  | "cloudOff";

export interface StatCardData {
  id: string;
  value: string;
  delta: string;
  trend: TrendDirection;
  label: string;
  caption: string;
}

export type RecommendationPriority = "high" | "medium" | "low";

export interface Recommendation {
  id: string;
  title: string;
  description: string;
  category: string;
  priority: RecommendationPriority;
  impact: string;
  meta: string;
}

export interface FixtureItem {
  id: string;
  opponent: string;
  competition: string;
  date: string;
  venue: "H" | "A";
  difficulty: 1 | 2 | 3 | 4 | 5;
}

export interface Performer {
  id: string;
  name: string;
  position: string;
  rating: number;
  form: number[];
  role: string;
}

export interface ActivityItem {
  id: string;
  title: string;
  detail: string;
  time: string;
  tone: "accent" | "info" | "warning" | "neutral";
}
