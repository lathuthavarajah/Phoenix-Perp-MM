"use client";

import clsx from "clsx";

interface Props {
  alerts: string[];
}

function alertStyle(alert: string): string {
  if (alert.startsWith("EMERGENCY_CLOSE"))
    return "bg-red-900/80 border-red-500 text-red-200";
  if (alert.startsWith("MARGIN_WARNING"))
    return "bg-yellow-900/80 border-yellow-500 text-yellow-200";
  if (alert.startsWith("REBALANCE_EXECUTED"))
    return "bg-blue-900/80 border-blue-500 text-blue-200";
  if (alert.startsWith("ENTRY_FAILED"))
    return "bg-orange-900/80 border-orange-500 text-orange-200";
  return "bg-gray-800 border-gray-600 text-gray-300";
}

export default function AlertBanner({ alerts }: Props) {
  if (alerts.length === 0) return null;

  const latest = alerts[alerts.length - 1];

  return (
    <div
      className={clsx(
        "w-full px-4 py-2 border-b text-sm font-mono",
        alertStyle(latest)
      )}
    >
      {latest}
    </div>
  );
}
