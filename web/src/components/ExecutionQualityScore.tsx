import type { Venue, ExecutionQualityScore as EQS } from '../types/market';
import { VENUES, VENUE_LABELS, VENUE_COLORS } from '../types/market';

interface Props {
  scores: Partial<Record<Venue, EQS>>;
}

function Gauge({ score, color, size = 100 }: { score: number; color: string; size?: number }) {
  const radius = (size - 10) / 2;
  const circumference = 2 * Math.PI * radius;
  const offset = circumference - (score / 100) * circumference;

  return (
    <svg width={size} height={size} className="transform -rotate-90">
      <circle
        cx={size / 2}
        cy={size / 2}
        r={radius}
        fill="none"
        stroke="#27272a"
        strokeWidth="6"
      />
      <circle
        cx={size / 2}
        cy={size / 2}
        r={radius}
        fill="none"
        stroke={color}
        strokeWidth="6"
        strokeDasharray={circumference}
        strokeDashoffset={offset}
        strokeLinecap="round"
        className="transition-all duration-700 ease-out"
      />
    </svg>
  );
}

function ScoreBar({ label, value, color }: { label: string; value: number; color: string }) {
  return (
    <div className="flex items-center gap-2 text-xs">
      <span className="w-14 text-zinc-500">{label}</span>
      <div className="h-1.5 flex-1 rounded-full bg-zinc-800">
        <div
          className="h-full rounded-full transition-all duration-500"
          style={{ width: `${Math.min(value, 100)}%`, backgroundColor: color }}
        />
      </div>
      <span className="w-8 text-right font-mono text-zinc-400">
        {value.toFixed(0)}
      </span>
    </div>
  );
}

export function ExecutionQualityScorePanel({ scores }: Props) {
  return (
    <div className="rounded-xl border border-zinc-800 bg-zinc-900 p-5">
      <h2 className="mb-4 text-lg font-semibold text-zinc-100">
        Execution Quality Score
      </h2>
      <div className="grid grid-cols-2 gap-6 lg:grid-cols-4">
        {VENUES.map((v) => {
          const s = scores[v];
          const color = VENUE_COLORS[v];
          const composite = s?.composite_score ?? 0;

          return (
            <div
              key={v}
              className={`flex flex-col items-center gap-3 rounded-lg p-4 ${
                v === 'phoenix' ? 'bg-orange-950/20 ring-1 ring-orange-800/30' : 'bg-zinc-800/30'
              }`}
            >
              <span
                className={`text-sm font-medium ${
                  v === 'phoenix' ? 'text-orange-400' : 'text-zinc-300'
                }`}
              >
                {VENUE_LABELS[v]}
              </span>
              <div className="relative">
                <Gauge score={composite} color={color} size={80} />
                <span className="absolute inset-0 flex items-center justify-center text-lg font-bold text-zinc-100">
                  {composite.toFixed(0)}
                </span>
              </div>
              <div className="w-full space-y-1">
                <ScoreBar label="Spread" value={s?.spread_score ?? 0} color={color} />
                <ScoreBar label="Depth" value={s?.depth_score ?? 0} color={color} />
                <ScoreBar label="Funding" value={s?.funding_score ?? 0} color={color} />
              </div>
            </div>
          );
        })}
      </div>
    </div>
  );
}
