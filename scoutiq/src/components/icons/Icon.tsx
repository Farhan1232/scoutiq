import type { ReactNode } from "react";
import type { IconName } from "../../types";

interface IconProps {
  name: IconName;
  size?: number;
  className?: string;
}

const paths: Record<IconName, ReactNode> = {
  dashboard: (
    <>
      <rect x="3" y="3" width="7" height="9" rx="1.5" />
      <rect x="14" y="3" width="7" height="5" rx="1.5" />
      <rect x="14" y="12" width="7" height="9" rx="1.5" />
      <rect x="3" y="16" width="7" height="5" rx="1.5" />
    </>
  ),
  squad: (
    <>
      <circle cx="9" cy="8" r="3" />
      <circle cx="16.5" cy="9" r="2.5" />
      <path d="M3.5 19.5c.8-3 2.8-4.5 5.5-4.5s4.7 1.5 5.5 4.5" />
      <path d="M14 19.5c.4-1.8 1.6-3 3.3-3.3" />
    </>
  ),
  transfers: (
    <>
      <path d="M7 7h10l-2.5-2.5" />
      <path d="M17 17H7l2.5 2.5" />
      <path d="M7 12h10" />
    </>
  ),
  tactics: (
    <>
      <rect x="5.5" y="2.5" width="13" height="19" rx="1.5" />
      <path d="M5.5 12h13" />
      <circle cx="12" cy="12" r="2.2" />
      <path d="M12 2.5v2.8M12 18.7v2.8" />
      <circle cx="8.2" cy="6.8" r="1" />
      <circle cx="15.8" cy="6.8" r="1" />
      <circle cx="8.2" cy="17.2" r="1" />
      <circle cx="15.8" cy="17.2" r="1" />
    </>
  ),
  analytics: (
    <>
      <path d="M4 19V5" />
      <path d="M4 19h16" />
      <path d="M8 15v-4" />
      <path d="M12 15V8" />
      <path d="M16 15v-7" />
    </>
  ),
  finances: (
    <>
      <circle cx="12" cy="12" r="8.5" />
      <path d="M12 7.5v9" />
      <path d="M14.5 9.5c0-1-1-1.75-2.5-1.75s-2.5.75-2.5 1.75 1 1.5 2.5 1.75 2.5.75 2.5 1.75-1 1.75-2.5 1.75-2.5-.75-2.5-1.75" />
    </>
  ),
  scouting: (
    <>
      <circle cx="11" cy="11" r="6.5" />
      <path d="M16 16l4.5 4.5" />
    </>
  ),
  settings: (
    <>
      <circle cx="12" cy="12" r="3" />
      <path d="M12 3.2 13.2 5.4l2.4-.3 1.2 2.1 2.1 1.2-.3 2.4L20.8 12l-2.2 1.2.3 2.4-2.1 1.2-1.2 2.1-2.4-.3L12 20.8l-1.2-2.2-2.4.3-1.2-2.1-2.1-1.2.3-2.4L3.2 12l2.2-1.2-.3-2.4 2.1-1.2 1.2-2.1 2.4.3L12 3.2Z" />
    </>
  ),
  search: (
    <>
      <circle cx="11" cy="11" r="6.5" />
      <path d="M16.2 16.2 20.5 20.5" />
    </>
  ),
  bell: (
    <>
      <path d="M6.5 16.5h11l-1.2-1.4a2 2 0 0 1-.5-1.3V11a4.3 4.3 0 1 0-8.6 0v2.8c0 .5-.18.96-.5 1.3L6.5 16.5Z" />
      <path d="M10 18.5a2 2 0 0 0 4 0" />
    </>
  ),
  chevronDown: <path d="M7 10l5 5 5-5" />,
  sparkles: (
    <>
      <path d="M12 4l1.2 3.8L17 9l-3.8 1.2L12 14l-1.2-3.8L7 9l3.8-1.2L12 4Z" />
      <path d="M18.5 13.5l.7 2.1 2.1.7-2.1.7-.7 2.1-.7-2.1-2.1-.7 2.1-.7.7-2.1Z" />
    </>
  ),
  arrowUpRight: (
    <>
      <path d="M7 17 17 7" />
      <path d="M9 7h8v8" />
    </>
  ),
  arrowDownRight: (
    <>
      <path d="M7 7l10 10" />
      <path d="M17 9v8h-8" />
    </>
  ),
  minus: <path d="M6 12h12" />,
  target: (
    <>
      <circle cx="12" cy="12" r="8.5" />
      <circle cx="12" cy="12" r="4.5" />
      <circle cx="12" cy="12" r="1.5" />
    </>
  ),
  calendar: (
    <>
      <rect x="3.5" y="5" width="17" height="15" rx="2" />
      <path d="M3.5 10h17" />
      <path d="M8 3.5v3" />
      <path d="M16 3.5v3" />
    </>
  ),
  command: (
    <>
      <path d="M8.5 8.5h7v7h-7z" />
      <path d="M6 8.5A2.5 2.5 0 1 1 8.5 6" />
      <path d="M15.5 6A2.5 2.5 0 1 1 18 8.5" />
      <path d="M18 15.5A2.5 2.5 0 1 1 15.5 18" />
      <path d="M8.5 18A2.5 2.5 0 1 1 6 15.5" />
    </>
  ),
  sun: (
    <>
      <circle cx="12" cy="12" r="4" />
      <path d="M12 3.5v1.8M12 18.7v1.8M3.5 12h1.8M18.7 12h1.8M6.1 6.1l1.3 1.3M16.6 16.6l1.3 1.3M17.9 6.1l-1.3 1.3M7.4 16.6l-1.3 1.3" />
    </>
  ),
  moon: (
    <path d="M19 14.5A7.5 7.5 0 1 1 9.5 5 6.2 6.2 0 0 0 19 14.5Z" />
  ),
  monitor: (
    <>
      <rect x="3.5" y="4.5" width="17" height="12" rx="2" />
      <path d="M8 20.5h8M12 16.5v4" />
    </>
  ),
  google: (
    <>
      <path d="M12 11.5v2.8h6.3c-.3 1.5-1.7 4.3-6.3 4.3A6.6 6.6 0 1 1 12 5.4c1.9 0 3.2.8 3.9 1.5l2.1-2C16.7 3.6 14.6 2.5 12 2.5A9.5 9.5 0 1 0 21.5 12c0-.6-.1-1.2-.2-1.7H12Z" />
    </>
  ),
  apple: (
    <path d="M16.6 12.7c0-2.2 1.8-3.2 1.9-3.3-1-1.5-2.7-1.7-3.3-1.7-1.4-.1-2.7.8-3.4.8-.7 0-1.8-.8-3-.8-1.5 0-2.9.9-3.7 2.3-1.6 2.7-.4 6.8 1.1 9 .8 1.1 1.7 2.3 2.9 2.2 1.2 0 1.6-.7 3-.7s1.8.7 3 .7 2-.1 2.9-2.3c.8-1.2 1.2-2.4 1.2-2.5-.1 0-2.3-.9-2.5-3.7ZM14.8 5.8c.6-.8 1.1-1.9.9-3-.9 0-2 .6-2.6 1.4-.6.7-1.1 1.8-.9 2.9 1 .1 2-.5 2.6-1.3Z" />
  ),
  mail: (
    <>
      <rect x="3.5" y="5.5" width="17" height="13" rx="2" />
      <path d="m4.5 7.5 7.5 6 7.5-6" />
    </>
  ),
  user: (
    <>
      <circle cx="12" cy="8.5" r="3.2" />
      <path d="M5 19c1.2-3.2 3.4-4.8 7-4.8s5.8 1.6 7 4.8" />
    </>
  ),
  logOut: (
    <>
      <path d="M10 4.5H6.5A2 2 0 0 0 4.5 6.5v11a2 2 0 0 0 2 2H10" />
      <path d="M14 12H21" />
      <path d="m17.5 8.5 3.5 3.5-3.5 3.5" />
    </>
  ),
  upload: (
    <>
      <path d="M12 16V6.5" />
      <path d="m8.5 9.5 3.5-3.5 3.5 3.5" />
      <path d="M5.5 17.5h13" />
    </>
  ),
  cloud: (
    <path d="M7.5 17.5h9.2a3.8 3.8 0 0 0 .3-7.6 5.2 5.2 0 0 0-10-1.4A3.7 3.7 0 0 0 7.5 17.5Z" />
  ),
  cloudOff: (
    <>
      <path d="M7.5 17.5h9.2a3.8 3.8 0 0 0 .3-7.6 5.2 5.2 0 0 0-2.2-.8" />
      <path d="M6.2 10A3.7 3.7 0 0 0 7.5 17.5" />
      <path d="m4 4 16 16" />
    </>
  ),
};

export function Icon({ name, size = 18, className }: IconProps) {
  return (
    <svg
      width={size}
      height={size}
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      strokeWidth="1.7"
      strokeLinecap="round"
      strokeLinejoin="round"
      className={className}
      aria-hidden="true"
    >
      {paths[name]}
    </svg>
  );
}
