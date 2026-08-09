import type {
  ActivityItem,
  FixtureItem,
  NavItem,
  Performer,
  RecommendationPriority,
  TrendDirection,
} from "../types";

export const NAV_ITEMS: NavItem[] = [
  { id: "dashboard", icon: "dashboard" },
  { id: "squad", icon: "squad" },
  { id: "transfers", icon: "transfers", badge: "3" },
  { id: "tactics", icon: "tactics" },
  { id: "analytics", icon: "analytics" },
  { id: "finances", icon: "finances" },
  { id: "scouting", icon: "scouting", badge: "12" },
  { id: "saveAnalyse", icon: "analytics", badge: "DEV" },
  { id: "settings", icon: "settings" },
];

export const STAT_META: Array<{ id: string; trend: TrendDirection }> = [
  { id: "squad-value", trend: "up" },
  { id: "form", trend: "up" },
  { id: "budget", trend: "down" },
  { id: "scout", trend: "up" },
];

export const REC_META: Array<{
  id: string;
  priority: RecommendationPriority;
}> = [
  { id: "r1", priority: "high" },
  { id: "r2", priority: "high" },
  { id: "r3", priority: "medium" },
  { id: "r4", priority: "medium" },
];

export const FIXTURE_META: Array<
  Pick<FixtureItem, "id" | "venue" | "difficulty">
> = [
  { id: "f1", venue: "A", difficulty: 5 },
  { id: "f2", venue: "H", difficulty: 3 },
  { id: "f3", venue: "H", difficulty: 3 },
  { id: "f4", venue: "A", difficulty: 2 },
];

export const PERFORMER_META: Array<
  Pick<Performer, "id" | "position" | "rating" | "form">
> = [
  {
    id: "p1",
    position: "AM",
    rating: 8.1,
    form: [7.4, 7.8, 8.6, 7.9, 8.1],
  },
  {
    id: "p2",
    position: "ST",
    rating: 7.8,
    form: [6.9, 7.5, 8.2, 7.6, 7.8],
  },
  {
    id: "p3",
    position: "LB",
    rating: 7.4,
    form: [7.2, 7.1, 7.6, 7.3, 7.4],
  },
  {
    id: "p4",
    position: "DM",
    rating: 7.3,
    form: [6.8, 7.0, 7.5, 7.4, 7.3],
  },
];

export const ACTIVITY_META: Array<Pick<ActivityItem, "id" | "tone">> = [
  { id: "a1", tone: "accent" },
  { id: "a2", tone: "warning" },
  { id: "a3", tone: "info" },
  { id: "a4", tone: "neutral" },
];

export const CLUB = {
  season: "2025/26",
  position: 4,
};
