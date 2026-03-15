import { useEffect, useReducer } from 'react';
import type {
  Venue,
  TradingPair,
  Orderbook,
  FundingRateSnapshot,
  ExecutionQualityScore,
  WsMsg,
} from '../types/market';

interface State {
  orderbooks: Partial<Record<Venue, Orderbook>>;
  funding: Partial<Record<Venue, FundingRateSnapshot>>;
  scores: Partial<Record<Venue, ExecutionQualityScore>>;
  connected: boolean;
}

type Action =
  | { type: 'OB'; data: Orderbook }
  | { type: 'FR'; data: FundingRateSnapshot }
  | { type: 'EQS'; data: ExecutionQualityScore[] }
  | { type: 'CONN'; connected: boolean };

function reducer(s: State, a: Action): State {
  switch (a.type) {
    case 'OB':
      return { ...s, orderbooks: { ...s.orderbooks, [a.data.venue]: a.data } };
    case 'FR':
      return { ...s, funding: { ...s.funding, [a.data.venue]: a.data } };
    case 'EQS': {
      const next = { ...s.scores };
      a.data.forEach((sc) => {
        next[sc.venue] = sc;
      });
      return { ...s, scores: next };
    }
    case 'CONN':
      return { ...s, connected: a.connected };
  }
}

export function useMarketFeed(pair: TradingPair) {
  const [state, dispatch] = useReducer(reducer, {
    orderbooks: {},
    funding: {},
    scores: {},
    connected: false,
  });

  useEffect(() => {
    const wsUrl = import.meta.env.VITE_WS_URL ?? 'ws://localhost:8080/ws';
    let ws: WebSocket;
    let reconnectTimer: ReturnType<typeof setTimeout>;

    const connect = () => {
      ws = new WebSocket(wsUrl);
      ws.onopen = () => dispatch({ type: 'CONN', connected: true });
      ws.onclose = () => {
        dispatch({ type: 'CONN', connected: false });
        reconnectTimer = setTimeout(connect, 2000);
      };
      ws.onmessage = ({ data }) => {
        const msg: WsMsg = JSON.parse(data);
        if (msg.type === 'orderbook' && msg.data.pair === pair) {
          dispatch({ type: 'OB', data: msg.data });
        } else if (msg.type === 'funding' && msg.data.pair === pair) {
          dispatch({ type: 'FR', data: msg.data });
        } else if (msg.type === 'scores') {
          dispatch({
            type: 'EQS',
            data: msg.data.filter((s) => s.pair === pair),
          });
        }
      };
    };

    connect();
    return () => {
      clearTimeout(reconnectTimer);
      ws?.close();
    };
  }, [pair]);

  return state;
}
