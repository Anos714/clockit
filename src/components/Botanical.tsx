import type { CSSProperties } from "react";

const LEAF_PATH = "M0 0C13 -15 32 -14 43 0 31 16 12 16 0 0Z";
const LEAF_VEIN = "M4 0C15 -1.5 25 -2 36 0";

const BRANCH_LEAVES = [
  { x: 42, y: 672, r: -34, s: 0.78 },
  { x: 92, y: 606, r: 42, s: 0.9 },
  { x: 143, y: 548, r: -58, s: 0.72 },
  { x: 177, y: 478, r: 38, s: 0.98 },
  { x: 209, y: 414, r: -44, s: 0.82 },
  { x: 240, y: 350, r: 48, s: 0.86 },
  { x: 274, y: 292, r: -52, s: 0.78 },
  { x: 315, y: 236, r: 34, s: 0.92 },
  { x: 356, y: 188, r: -26, s: 0.74 },
  { x: 414, y: 142, r: 18, s: 0.66 },
  { x: 462, y: 102, r: -8, s: 0.54 },
  { x: 100, y: 546, r: -86, s: 0.64 },
  { x: 58, y: 516, r: -122, s: 0.58 },
  { x: 194, y: 380, r: 92, s: 0.62 },
  { x: 224, y: 340, r: 12, s: 0.72 },
  { x: 330, y: 214, r: -78, s: 0.58 },
  { x: 376, y: 250, r: 26, s: 0.6 },
];

export function LeafGlyph({
  size = 18,
  className,
}: {
  size?: number;
  className?: string;
}) {
  return (
    <svg
      className={className}
      width={size}
      height={size}
      viewBox="0 0 24 24"
      aria-hidden="true"
    >
      <path
        d="M4 20C3 12 7 5 21 4c-.5 12-6.5 16-17 16Z"
        fill="currentColor"
      />
      <path
        d="M5 19C9 15 12 12 17 9"
        stroke="var(--leaf-vein, rgba(0, 0, 0, 0.22))"
        strokeWidth="0.9"
        strokeLinecap="round"
        fill="none"
      />
    </svg>
  );
}

export function SproutMark({
  size = 40,
  className = "",
}: {
  size?: number;
  className?: string;
}) {
  return (
    <span
      className={`sprout-mark ${className}`.trim()}
      style={{ width: size, height: size }}
      aria-hidden="true"
    >
      <svg viewBox="0 0 48 48" fill="none">
        <circle className="sprout-ring" cx="24" cy="24" r="22.4" />
        <path className="sprout-stem" d="M24 37.5V21" />
        <path
          className="sprout-leaf"
          d="M24 30.2C17.8 31.8 13.2 27.4 13.2 20c7.2-1.1 10.8 3.3 10.8 10.2Z"
        />
        <path
          className="sprout-leaf"
          d="M24 25.2c.4-7.7 5.2-11.8 12.3-11.8 0 7.6-4.9 11.9-12.3 11.8Z"
        />
        <path
          className="sprout-vein"
          d="M24 30.1c-2.7-3.1-4.3-6-4.8-8.7M24 25.1c3-2.6 5.6-4.5 7.8-5.7"
        />
        <circle className="sprout-dot" cx="36.2" cy="12.2" r="2.1" />
      </svg>
    </span>
  );
}

function BranchSvg() {
  return (
    <svg className="branch-svg" viewBox="0 0 520 760" fill="none">
      <path
        className="branch-stem"
        pathLength={1}
        d="M-30 726C96 666 168 566 201 454c27-91 92-189 182-252 42-30 85-53 131-67"
      />
      <path
        className="branch-stem stem-thin"
        pathLength={1}
        d="M143 566C112 500 72 454 18 424"
      />
      <path
        className="branch-stem stem-thin"
        pathLength={1}
        d="M232 386c57-29 96-66 122-118"
      />
      <path
        className="branch-stem stem-thin"
        pathLength={1}
        d="M320 232c40 24 82 32 128 26"
      />
      <g className="branch-leaves">
        {BRANCH_LEAVES.map((leaf, index) => (
          <g
            key={index}
            transform={`translate(${leaf.x} ${leaf.y}) rotate(${leaf.r}) scale(${leaf.s})`}
          >
            <path d={LEAF_PATH} />
            <path className="leaf-vein" d={LEAF_VEIN} />
          </g>
        ))}
      </g>
      <circle className="branch-berry" cx="268" cy="316" r="3.2" />
      <circle className="branch-berry" cx="302" cy="250" r="2.4" />
    </svg>
  );
}

const DRIFT_LEAVES = [
  { left: 6, delay: -4, duration: 38, scale: 0.78, drift: 38 },
  { left: 21, delay: -19, duration: 46, scale: 0.5, drift: -22 },
  { left: 45, delay: -9, duration: 42, scale: 0.66, drift: 30 },
  { left: 63, delay: -28, duration: 52, scale: 0.86, drift: -34 },
  { left: 78, delay: -14, duration: 36, scale: 0.52, drift: 20 },
  { left: 91, delay: -33, duration: 48, scale: 0.7, drift: -18 },
];

const FIREFLIES = [
  { left: "18%", top: "34%", delay: 0, duration: 9 },
  { left: "32%", top: "62%", delay: 2.4, duration: 11 },
  { left: "71%", top: "28%", delay: 1.2, duration: 8 },
  { left: "84%", top: "57%", delay: 3.6, duration: 12 },
  { left: "56%", top: "78%", delay: 5.1, duration: 10 },
];

export function ForestBackdrop({
  variant = "app",
}: {
  variant?: "app" | "focus";
}) {
  return (
    <div className={`forest-backdrop ${variant}`} aria-hidden="true">
      <div className="branch-wrap branch-left">
        <BranchSvg />
      </div>
      <div className="branch-wrap branch-right">
        <BranchSvg />
      </div>
      <div className="leaf-field">
        {DRIFT_LEAVES.map((leaf, index) => (
          <span
            key={index}
            className="drift-leaf"
            style={
              {
                "--left": `${leaf.left}%`,
                "--delay": `${leaf.delay}s`,
                "--duration": `${leaf.duration}s`,
                "--scale": leaf.scale,
                "--drift": `${leaf.drift}px`,
              } as CSSProperties
            }
          >
            <LeafGlyph size={20} />
          </span>
        ))}
      </div>
      <div className="fireflies">
        {FIREFLIES.map((fly, index) => (
          <span
            key={index}
            className="firefly"
            style={
              {
                "--left": fly.left,
                "--top": fly.top,
                "--delay": `${fly.delay}s`,
                "--duration": `${fly.duration}s`,
              } as CSSProperties
            }
          />
        ))}
      </div>
    </div>
  );
}

export function CornerSprig({ className = "" }: { className?: string }) {
  return (
    <svg
      className={`corner-sprig ${className}`.trim()}
      viewBox="0 0 150 120"
      fill="none"
      aria-hidden="true"
    >
      <path
        className="sprig-stem"
        d="M4 118C32 92 64 68 103 49c15-7 29-12 44-15"
      />
      <g className="sprig-leaves">
        <path d="M38 91c-2-13 4-22 16-26 2 13-4 22-16 26Z" />
        <path d="M65 70c3-12 11-18 23-18-2 12-11 19-23 18Z" />
        <path d="M94 53c-1-11 5-19 16-22 1 11-5 19-16 22Z" />
        <path d="M78 61c7-9 16-12 27-8-5 10-15 13-27 8Z" />
        <path d="M119 39c1-9 6-15 15-17 0 9-5 15-15 17Z" />
        <path d="M112 44c-8-5-16-4-23 3 8 5 16 4 23-3Z" />
      </g>
    </svg>
  );
}

export function FocusGrove() {
  return (
    <div className="focus-grove" aria-hidden="true">
      <svg className="focus-branch focus-branch-left" viewBox="0 0 640 900" fill="none">
        <path
          className="focus-stem"
          d="M-40 900C70 760 157 648 220 516c49-104 130-190 238-246 50-25 102-43 158-52"
        />
        <path className="focus-stem thin" d="M128 700C92 620 44 566-24 536" />
        <path className="focus-stem thin" d="M248 460c66-30 110-72 138-132" />
        <g className="focus-leaves">
          <path d="M56 814c2-44 24-70 64-76-1 42-25 68-64 76Z" transform="rotate(-20 88 776)" />
          <path d="M142 672c-6-38 12-65 48-74 3 38-14 65-48 74Z" transform="rotate(8 166 635)" />
          <path d="M212 536c-2-36 16-61 51-68 1 35-17 61-51 68Z" transform="rotate(-6 236 502)" />
          <path d="M300 408c10-32 34-49 66-44-9 31-33 48-66 44Z" transform="rotate(10 332 386)" />
          <path d="M414 297c14-28 38-40 67-30-14 27-38 39-67 30Z" transform="rotate(14 444 283)" />
          <path d="M110 662c-26-22-52-26-78-9 24 22 51 26 78 9Z" transform="rotate(-6 72 660)" />
          <path d="M250 452c-6-30 8-53 38-63 4 30-11 52-38 63Z" transform="rotate(28 262 421)" />
        </g>
      </svg>
      <svg className="focus-branch focus-branch-right" viewBox="0 0 640 900" fill="none">
        <path
          className="focus-stem"
          d="M680 900c-112-150-210-264-288-382-61-91-156-163-272-205-45-17-90-27-136-31"
        />
        <path className="focus-stem thin" d="M392 610c50-62 108-100 178-116" />
        <g className="focus-leaves">
          <path d="M578 800c-2-44-24-70-64-76 1 42 25 68 64 76Z" transform="rotate(20 546 762)" />
          <path d="M480 650c6-38-12-65-48-74-3 38 14 65 48 74Z" transform="rotate(-9 456 613)" />
          <path d="M396 512c2-36-16-61-51-68-1 35 17 61 51 68Z" transform="rotate(7 370 478)" />
          <path d="M300 380c-10-32-34-49-66-44 9 31 33 48 66 44Z" transform="rotate(-11 268 358)" />
          <path d="M448 540c26-22 52-26 78-9-24 22-51 26-78 9Z" transform="rotate(8 486 538)" />
        </g>
      </svg>
      <div className="focus-mist mist-a" />
      <div className="focus-mist mist-b" />
    </div>
  );
}
