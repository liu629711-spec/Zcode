// 上票对话框（作者通道）：acceptance_criteria 非空是票据进看板的唯一准入谓词；AlertDialog 惯例同看板确认框。
import {
  AlertDialog,
  AlertDialogAction,
  AlertDialogCancel,
  AlertDialogContent,
  AlertDialogDescription,
  AlertDialogFooter,
  AlertDialogHeader,
  AlertDialogTitle,
} from "@/components/ui/alert-dialog.js";
import { Textarea } from "@/components/ui/textarea.js";
import { useZCodeIntl } from "@/i18n/IntlProvider.js";

export function TaskSpecDialog({
  open,
  value,
  onOpenChange,
  onChange,
  onConfirm,
}: {
  open: boolean;
  value: string;
  onOpenChange: (open: boolean) => void;
  onChange: (value: string) => void;
  onConfirm: () => void;
}) {
  const { intl } = useZCodeIntl();
  return (
    <AlertDialog open={open} onOpenChange={onOpenChange}>
      <AlertDialogContent>
        <AlertDialogHeader>
          <AlertDialogTitle>
            {intl.formatMessage({ id: "dispatchDesk.writeSpec" })}
          </AlertDialogTitle>
          <AlertDialogDescription>
            {intl.formatMessage({ id: "dispatchDesk.writeSpec.hint" })}
          </AlertDialogDescription>
        </AlertDialogHeader>
        <Textarea
          value={value}
          rows={5}
          onChange={(event) => {
            onChange(event.target.value);
          }}
        />
        <AlertDialogFooter>
          <AlertDialogCancel type="button" size="sm">
            {intl.formatMessage({ id: "common.cancel" })}
          </AlertDialogCancel>
          <AlertDialogAction
            type="button"
            size="sm"
            disabled={value.trim().length === 0}
            onClick={onConfirm}
          >
            {intl.formatMessage({ id: "common.confirm" })}
          </AlertDialogAction>
        </AlertDialogFooter>
      </AlertDialogContent>
    </AlertDialog>
  );
}
