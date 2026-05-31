# Progress

## Status
In Progress

## Tasks Completed
- Fixed auto-lock reset: `updateActivity` now calls `setIsLocked(false)` before `setLastActivity(Date.now())` to prevent re-triggering

## Files Changed
- `src/hooks/useAutoLock.ts` — swapped order of reset + timestamp in `updateActivity` callback

## Notes
- The `isLockedRef` pattern was already in place, but the reset happened after the timestamp update. Swapping the order ensures the interval won't re-trigger between state updates.
