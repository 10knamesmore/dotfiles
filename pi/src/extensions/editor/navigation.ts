/** Optional agent-widget navigation, supplied by the workflow extension. */
export interface AgentInputNavigation {
  selectNext(): boolean;
  selectPrevious(): void;
  hasSelection(): boolean;
  openSelection(): boolean;
  clearSelection(): void;
}
