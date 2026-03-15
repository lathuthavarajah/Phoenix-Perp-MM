interface Props {
  connected: boolean;
}

export function LiveIndicator({ connected }: Props) {
  return (
    <div className="flex items-center gap-2">
      <div
        className={`h-2.5 w-2.5 rounded-full ${
          connected
            ? 'bg-green-500 shadow-[0_0_6px_rgba(34,197,94,0.6)]'
            : 'bg-red-500 animate-pulse'
        }`}
      />
      <span className="text-xs text-zinc-400">
        {connected ? 'Live' : 'Connecting...'}
      </span>
    </div>
  );
}
