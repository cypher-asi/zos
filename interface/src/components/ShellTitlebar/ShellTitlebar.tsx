import { type ReactNode, type HTMLAttributes } from "react";
import { Topbar } from "@cypher-asi/zui";
import { windowCommand } from "../../shell/platform";
import styles from "./ShellTitlebar.module.css";

export interface ShellTitlebarProps
  extends Omit<HTMLAttributes<HTMLElement>, "children" | "title"> {
  icon?: ReactNode;
  title: ReactNode;
  actions?: ReactNode;
}

/**
 * Wraps the zui `Topbar` with the floating-pill chrome (drag region,
 * inset rounded background, frosted blur). Owns the `--shell-chrome-*`
 * tokens shared with `BottomTaskbar` so both pills resolve identical
 * height/inset/radius/background.
 */
export function ShellTitlebar({
  icon,
  title,
  actions,
  className,
  onDoubleClick,
  ...rest
}: ShellTitlebarProps) {
  const composedClassName = [
    "titlebar-drag",
    styles.alignRail,
    styles.blur,
    className,
  ]
    .filter(Boolean)
    .join(" ");

  return (
    <Topbar
      className={composedClassName}
      onDoubleClick={onDoubleClick ?? (() => windowCommand("maximize"))}
      icon={icon}
      title={title}
      actions={actions}
      {...rest}
    />
  );
}
