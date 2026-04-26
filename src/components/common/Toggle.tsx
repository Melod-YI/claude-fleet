import { cn } from "@/lib/utils"

interface ToggleProps {
  checked: boolean
  onChange: (checked: boolean) => void
  label?: string
  className?: string
}

export function Toggle({ checked, onChange, label, className }: ToggleProps) {
  return (
    <div className={cn("flex items-center gap-2", className)}>
      {label && <span className="text-sm text-gray-600">{label}</span>}
      <button
        onClick={() => onChange(!checked)}
        className={cn(
          "w-11 h-6 rounded-full relative transition-colors",
          checked ? "bg-violet-600" : "bg-gray-300"
        )}
      >
        <span
          className={cn(
            "absolute w-5 h-5 bg-white rounded-full top-0.5 transition-transform",
            checked ? "translate-x-5" : "translate-x-0.5"
          )}
        />
      </button>
    </div>
  )
}