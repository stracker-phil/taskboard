import {
  AlarmClock, Bookmark, Bug, Calendar, CheckCircle2, CircleDot, Code2, Coffee, Compass, Flag, Flame, FlaskConical,
  Folder, Funnel, Gem, Globe, Heart, Inbox, Layers, Lightbulb, ListChecks, Lock, Mail, Megaphone, Package, Palette,
  Rocket, Search, Shield, ShoppingCart, Sparkles, Star, Target, Timer, TrendingUp, Truck, Users, Wrench, Zap,
  type LucideIcon,
} from "lucide-react";

/** The line icons a tile can wear, by the name stored with it (Spaces, filter lists, views; see `look.rs`). */
export const LIST_ICONS: Record<string, LucideIcon> = {
  funnel: Funnel, star: Star, flag: Flag, bug: Bug, rocket: Rocket, flame: Flame, zap: Zap, target: Target,
  inbox: Inbox, "list-checks": ListChecks, "check-circle": CheckCircle2, "circle-dot": CircleDot, calendar: Calendar,
  timer: Timer, "alarm-clock": AlarmClock, bookmark: Bookmark, heart: Heart, gem: Gem, sparkles: Sparkles,
  lightbulb: Lightbulb, megaphone: Megaphone, users: Users, mail: Mail, code: Code2, wrench: Wrench,
  "flask-conical": FlaskConical, shield: Shield, lock: Lock, package: Package, truck: Truck, "shopping-cart": ShoppingCart,
  globe: Globe, compass: Compass, layers: Layers, folder: Folder, palette: Palette, "trending-up": TrendingUp,
  coffee: Coffee, search: Search,
};

/** A list's icon; the funnel when it has none or a name this build doesn't know. */
export const listIcon = (name: string | null | undefined): LucideIcon => (name ? LIST_ICONS[name] : undefined) ?? Funnel;

/** A stored icon that names a line icon this build knows; anything else is a letter or an emoji. */
export const lineIcon = (icon: string | null | undefined): LucideIcon | null => (icon && icon in LIST_ICONS ? LIST_ICONS[icon] : null);
