import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from "@/components/ui/select"
import type { SessionFilter } from "@/types"

interface TimeRangeSelectProps {
  value: SessionFilter['timeRange']
  onChange: (value: SessionFilter['timeRange']) => void
}

type TimeRangeValue = '3d' | '7d' | '30d' | 'all'

const TIME_RANGE_OPTIONS: { value: TimeRangeValue; label: string }[] = [
  { value: '3d', label: '近 3 天' },
  { value: '7d', label: '近 7 天' },
  { value: '30d', label: '近 30 天' },
  { value: 'all', label: '全部时间' },
]

export function TimeRangeSelect({ value, onChange }: TimeRangeSelectProps) {
  const handleValueChange = (val: string) => {
    onChange(val as TimeRangeValue)
  }

  return (
    <Select value={value || '30d'} onValueChange={handleValueChange}>
      <SelectTrigger className="w-[120px] h-8">
        <SelectValue placeholder="选择时间范围" />
      </SelectTrigger>
      <SelectContent>
        {TIME_RANGE_OPTIONS.map((option) => (
          <SelectItem key={option.value} value={option.value}>
            {option.label}
          </SelectItem>
        ))}
      </SelectContent>
    </Select>
  )
}