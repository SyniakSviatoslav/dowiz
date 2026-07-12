import { type ReactNode } from 'react';
import { ResponsiveDialog } from './ResponsiveDialog.js';

export interface PickerOption {
  value: string;
  label: string;
  description?: string;
  icon?: string;
}

interface MobilePickerProps {
  open: boolean;
  onClose: () => void;
  options: PickerOption[];
  onSelect: (option: PickerOption) => void;
  title?: string;
  selectedValue?: string;
}

export function MobilePicker({ open, onClose, options, onSelect, title, selectedValue }: MobilePickerProps) {
  const list = (
    <div className="space-y-1">
      {options.map(opt => {
        const isSelected = opt.value === selectedValue;
        return (
          <button
            key={opt.value}
            onClick={() => {
              onSelect(opt);
              onClose();
            }}
            className={`w-full flex items-center gap-3 px-4 py-3 rounded-lg text-left transition-colors
              ${isSelected ? 'bg-brand-primary-light text-brand-primary' : 'hover:bg-brand-surface-raised text-brand-text'}
            `}
          >
            {opt.icon && <i className={`${opt.icon} text-lg shrink-0 ${isSelected ? 'text-brand-primary' : 'text-brand-text-muted'}`} />}
            <div className="flex-1 min-w-0">
              <div className="text-sm font-medium truncate">{opt.label}</div>
              {opt.description && <div className="text-xs text-brand-text-muted truncate">{opt.description}</div>}
            </div>
            {isSelected && <i className="ti ti-check text-brand-primary text-lg shrink-0" />}
          </button>
        );
      })}
    </div>
  );

  return (
    <ResponsiveDialog open={open} onClose={onClose} title={title}>
      {list}
    </ResponsiveDialog>
  );
}
