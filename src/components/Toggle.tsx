interface ToggleProps {
  checked: boolean;
  onChange: (value: boolean) => void;
  label: string;
  description: string;
}

export function Toggle({ checked, onChange, label, description }: ToggleProps) {
  return (
    <button
      type="button"
      className={`toggle-row${checked ? ' toggle-row-active' : ''}`}
      onClick={() => onChange(!checked)}
    >
      <div>
        <strong>{label}</strong>
        <p>{description}</p>
      </div>
      <span className={`toggle${checked ? ' toggle-active' : ''}`}>
        <span className="toggle-knob" />
      </span>
    </button>
  );
}
