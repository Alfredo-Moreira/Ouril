/**
 * The board (ADR 0021): the 3D board, its loading widget, and its hidden keyboard and
 * screen-reader controls.
 *
 * - The 3D board is a separate, lazily loaded chunk (preloaded while the browser is idle).
 *   Until it has really drawn, a small loading widget (`BoardLoader`) holds its place.
 * - Accessibility never depends on WebGL: `BoardControls` gives keyboard and screen-reader users
 *   the pits as visually hidden controls. Focusing one lights the same pit in the 3D view.
 * - If the 3D board can't run (no WebGL2) or fails to start (a lost context, the chunk failing
 *   to load), the board area says so. The hidden controls keep the game playable.
 */
import {
  Component,
  lazy,
  Suspense,
  useEffect,
  useState,
  type CSSProperties,
  type ReactNode,
} from 'react';
import { useTranslation } from 'react-i18next';

import { loadBoard3D, preloadBoard3D } from '../board3d/load';
import { detectDevice } from '../board3d/mode';
import { BoardControls, type BoardProps } from './BoardControls';

const Board3D = lazy(loadBoard3D);

// --- View --------------------------------------------------------------------------------------

export interface BoardViewProps extends BoardProps {
  /** Animation speed of the game (0 = none). */
  speed?: number;
  finished?: boolean;
}

export function BoardView(props: BoardViewProps) {
  const [device] = useState(detectDevice);
  const [failed, setFailed] = useState(false);
  const [ready, setReady] = useState(false);
  const [focusPit, setFocusPit] = useState<number | null>(null);

  useEffect(() => {
    if (device.webgl) preloadBoard3D();
  }, [device.webgl]);

  const { speed, finished, ...boardProps } = props;
  const problem = !device.webgl ? 'no_webgl' : failed ? 'failed' : null;

  return (
    <div className={`board3d${ready && !problem ? ' board3d--ready' : ''}`}>
      <BoardControls {...boardProps} onFocusPit={setFocusPit} />
      {problem ? (
        <BoardError problem={problem} />
      ) : (
        <>
          {/* The loader stays until the 3D board has drawn, then fades out over it. */}
          <BoardLoader />
          <Fallback onError={() => setFailed(true)}>
            <Suspense fallback={null}>
              <div className="board3d__stage">
                <Board3D
                  pits={props.pits}
                  stores={props.stores}
                  legal={props.legal}
                  onPlay={props.onPlay}
                  highlight={props.highlight}
                  hintPit={props.hintPit}
                  focusPit={focusPit}
                  speed={speed}
                  finished={finished}
                  onReady={() => setReady(true)}
                />
              </div>
            </Suspense>
          </Fallback>
        </>
      )}
    </div>
  );
}

/** In the board's place when the 3D board can't run or start. */
function BoardError({ problem }: { problem: 'no_webgl' | 'failed' }) {
  const { t } = useTranslation();
  return (
    <div className="board-error" role="status">
      <p>{t(problem === 'no_webgl' ? 'board.no_webgl' : 'board.error')}</p>
      {problem === 'failed' && (
        <button
          type="button"
          className="button button--on-dark"
          onClick={() => window.location.reload()}
        >
          {t('board.retry')}
        </button>
      )}
    </div>
  );
}

/** While the 3D board loads: ouri seeds dropping one by one into a bowl, and a label. */
function BoardLoader() {
  const { t } = useTranslation();
  return (
    <div className="board-loader" role="status">
      <div className="sow-loader" aria-hidden="true">
        <span className="sow-loader__bowl" />
        {[0, 1, 2].map((i) => (
          <span key={i} className="sow-loader__seed" style={{ '--i': i } as CSSProperties} />
        ))}
      </div>
      <p className="board-loader__label">{t('board.loading')}</p>
    </div>
  );
}

/** Any error inside the 3D view (no WebGL context, context lost, chunk failed). */
class Fallback extends Component<{ children: ReactNode; onError: () => void }, { error: boolean }> {
  override state = { error: false };
  static getDerivedStateFromError() {
    return { error: true };
  }
  override componentDidCatch(error: unknown) {
    console.warn('3D board unavailable', error);
    this.props.onError();
  }
  override render() {
    return this.state.error ? null : this.props.children;
  }
}
