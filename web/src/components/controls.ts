export const PRIMARY_BUTTON = "rounded-control bg-slate-900 px-3 py-1.5 text-sm font-medium text-white hover:bg-slate-700 disabled:opacity-60 dark:bg-slate-100 dark:text-slate-900 dark:hover:bg-white";

export const SECONDARY_BUTTON = "flex items-center gap-1.5 rounded-control bg-raised px-2.5 py-1 text-sm text-slate-700 hover:bg-raised-hover disabled:opacity-60 dark:text-slate-300";

export const ICON_BUTTON = "flex size-7 shrink-0 items-center justify-center rounded-control text-slate-500 hover:bg-raised hover:text-slate-900 disabled:opacity-60 dark:text-slate-400 dark:hover:text-slate-100";

export const DANGER_ICON_BUTTON = "flex size-7 shrink-0 items-center justify-center rounded-control text-slate-500 hover:bg-raised hover:text-red-600 dark:text-slate-400 dark:hover:text-red-400";

export const FIELD = "w-full rounded-control bg-raised px-3 py-1.5 text-sm text-slate-900 placeholder:text-slate-400 dark:text-slate-100 dark:placeholder:text-slate-500";

export const FIELD_LABEL = "block text-xs font-medium text-slate-600 dark:text-slate-400";

export const SECTION_HEADING = "text-sm font-semibold text-slate-700 dark:text-slate-300";

export const EMPTY_PANEL = "rounded-panel bg-surface px-4 py-8 text-center text-sm text-slate-500 dark:text-slate-500";

export const readText = (fields: FormData, name: string) => {
  const value = fields.get(name);

  return typeof value === "string" ? value.trim() : "";
};
