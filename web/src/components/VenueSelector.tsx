import type { TradingPair } from '../types/market';

interface Props {
  pair: TradingPair;
  onPairChange: (pair: TradingPair) => void;
}

const PAIRS: TradingPair[] = ['SOL-PERP', 'BTC-PERP'];

export function VenueSelector({ pair, onPairChange }: Props) {
  return (
    <div className="flex gap-2">
      {PAIRS.map((p) => (
        <button
          key={p}
          onClick={() => onPairChange(p)}
          className={`rounded-lg px-4 py-2 text-sm font-medium transition-colors ${
            pair === p
              ? 'bg-orange-600 text-white'
              : 'bg-zinc-800 text-zinc-400 hover:bg-zinc-700 hover:text-zinc-200'
          }`}
        >
          {p}
        </button>
      ))}
    </div>
  );
}
