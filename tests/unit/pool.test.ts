import { describe, expect, it } from 'vitest';
import { ObjectPool } from '../../src/core/pool.js';

interface Slot {
  readonly index: number;
  value: number;
}

function makePool(capacity: number): ObjectPool<Slot> {
  return new ObjectPool<Slot>(
    capacity,
    (index) => ({ index, value: 0 }),
    (item) => {
      item.value = 0;
    },
  );
}

describe('ObjectPool', () => {
  it('legt alle Objekte vorab an', () => {
    let created = 0;
    const pool = new ObjectPool<Slot>(
      16,
      (index) => {
        created++;
        return { index, value: 0 };
      },
      (item) => {
        item.value = 0;
      },
    );

    expect(created).toBe(16);
    for (let i = 0; i < 16; i++) pool.acquire();
    // Beim Vergeben entsteht nichts Neues — das ist der ganze Zweck.
    expect(created).toBe(16);
  });

  it('gibt zurückgesetzte Objekte aus', () => {
    const pool = makePool(4);
    const first = pool.acquire();
    first.value = 42;
    pool.release(first.index);
    const again = pool.acquire();

    expect(again.value).toBe(0);
  });

  it('zählt aktive Objekte richtig', () => {
    const pool = makePool(4);
    const a = pool.acquire();
    const b = pool.acquire();
    expect(pool.activeCount).toBe(2);
    pool.release(a.index);
    expect(pool.activeCount).toBe(1);
    pool.release(b.index);
    expect(pool.activeCount).toBe(0);
  });

  it('verwendet bei Erschöpfung das älteste belegte Objekt wieder', () => {
    const pool = makePool(3);
    const a = pool.acquire();
    const b = pool.acquire();
    const c = pool.acquire();
    a.value = 1;
    b.value = 2;
    c.value = 3;

    const recycled = pool.acquire();

    expect(recycled.index).toBe(a.index);
    expect(recycled.value).toBe(0);
    expect(pool.recycleCount).toBe(1);
    expect(pool.activeCount).toBe(3);

    // Der nächste Griff holt das zweitälteste.
    expect(pool.acquire().index).toBe(b.index);
  });

  it('übersteht doppeltes Freigeben', () => {
    const pool = makePool(2);
    const a = pool.acquire();
    pool.release(a.index);
    pool.release(a.index);
    pool.release(-1);
    pool.release(99);

    expect(pool.activeCount).toBe(0);
    expect(pool.acquire().index).toBeGreaterThanOrEqual(0);
  });

  it('gibt mit releaseAll alles frei', () => {
    const pool = makePool(4);
    pool.acquire();
    pool.acquire();
    pool.releaseAll();

    expect(pool.activeCount).toBe(0);
    for (let i = 0; i < 4; i++) expect(pool.isActive(i)).toBe(false);
  });

  it('bleibt über viele Umläufe stimmig', () => {
    const pool = makePool(8);
    for (let round = 0; round < 10_000; round++) {
      const item = pool.acquire();
      item.value = round;
      if (round % 3 === 0) pool.release(item.index);
    }

    expect(pool.activeCount).toBeLessThanOrEqual(8);
    expect(pool.capacity).toBe(8);
  });

  it('weist unsinnige Größen zurück', () => {
    expect(() => makePool(0)).toThrow(RangeError);
    expect(() => makePool(-3)).toThrow(RangeError);
  });
});
