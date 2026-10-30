import {
  Accessibility, AlarmClock, Archive, Ban, BarChart3, Blocks, BookOpen, Bookmark, Bug, Building2, Calendar, CheckCircle2,
  CircleDot, Code2, Coffee, Compass, CreditCard, Database, Eye, Flag, Flame, FlaskConical, Folder, Funnel, Gauge, Gem,
  GitBranch, GitPullRequest, Globe, Heart, Hourglass, Inbox, Kanban, KeyRound, Languages, Layers, LifeBuoy, Lightbulb,
  ListChecks, Lock, Mail, Megaphone, Milestone, Mountain, Package, Paintbrush, Palette, Percent, Plug, Puzzle, Receipt,
  Repeat, Rocket, Search, Server, Shield, ShieldAlert, ShoppingBag, ShoppingCart, Siren, Smartphone, Sparkles, Star,
  Store, Tag, Target, TestTube, Timer, TrendingUp, Truck, Users, Webhook, Wrench, Zap,
  type LucideIcon,
} from "lucide-react";

/** The line icons a tile can wear, by the name stored with it (Spaces, filter lists, views; see `look.rs`). The
 *  picker shows them in this order: general marks, then grouped by theme. */
export const LIST_ICONS: Record<string, LucideIcon> = {
  folder: Folder, funnel: Funnel, star: Star, flag: Flag, bug: Bug, rocket: Rocket, flame: Flame, zap: Zap, target: Target,
  inbox: Inbox, "list-checks": ListChecks, "check-circle": CheckCircle2, "circle-dot": CircleDot, calendar: Calendar,
  timer: Timer, "alarm-clock": AlarmClock, bookmark: Bookmark, heart: Heart, gem: Gem, sparkles: Sparkles,
  lightbulb: Lightbulb, megaphone: Megaphone, users: Users, mail: Mail, code: Code2, wrench: Wrench,
  "flask-conical": FlaskConical, shield: Shield, lock: Lock, package: Package, truck: Truck, "shopping-cart": ShoppingCart,
  globe: Globe, compass: Compass, layers: Layers, palette: Palette, "trending-up": TrendingUp, coffee: Coffee,
  search: Search,
  // Workflow
  eye: Eye, "git-pull-request": GitPullRequest, "git-branch": GitBranch, repeat: Repeat, kanban: Kanban,
  milestone: Milestone, mountain: Mountain, archive: Archive, tag: Tag, hourglass: Hourglass, ban: Ban, siren: Siren,
  // Quality
  "shield-alert": ShieldAlert, key: KeyRound, "test-tube": TestTube, gauge: Gauge, accessibility: Accessibility,
  // Shop
  store: Store, "shopping-bag": ShoppingBag, receipt: Receipt, "credit-card": CreditCard, percent: Percent,
  building: Building2, "life-buoy": LifeBuoy,
  // WordPress
  plug: Plug, puzzle: Puzzle, blocks: Blocks, paintbrush: Paintbrush, languages: Languages, "book-open": BookOpen,
  // Tech
  database: Database, server: Server, webhook: Webhook, smartphone: Smartphone, "chart-bar": BarChart3,
};

/** The icon a filter list shows when it has none or a name this build doesn't know. */
export const DEFAULT_LIST_ICON: LucideIcon = Folder;

/** A list's icon; the folder when it has none or a name this build doesn't know. */
export const listIcon = (name: string | null | undefined): LucideIcon => (name ? LIST_ICONS[name] : undefined) ?? DEFAULT_LIST_ICON;

/** A stored icon that names a line icon this build knows; anything else is a letter or an emoji. */
export const lineIcon = (icon: string | null | undefined): LucideIcon | null => (icon && icon in LIST_ICONS ? LIST_ICONS[icon] : null);
