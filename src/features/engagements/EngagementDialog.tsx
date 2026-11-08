import { useState } from "react";
import { useNavigate } from "react-router-dom";
import { useMutation, useQueryClient } from "@tanstack/react-query";
import { Dialog } from "@/components/ui/dialog";
import { Button } from "@/components/ui/button";
import { Spinner } from "@/components/ui/states";
import { api } from "@/lib/api";
import type { EngagementDetail, EngagementInput } from "@/types";
import { EngagementForm } from "@/features/engagements/EngagementForm";
import {
  emptyEngagementValues,
  valuesFromDetail,
} from "@/features/engagements/engagementFormModel";

export function EngagementDialog({
  open,
  onOpenChange,
  detail,
}: {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  detail?: EngagementDetail | null;
}) {
  const queryClient = useQueryClient();
  const navigate = useNavigate();
  const [serverError, setServerError] = useState<string | null>(null);
  const editing = Boolean(detail);
  const formId = "engagement-form";

  const mutation = useMutation({
    mutationFn: (input: EngagementInput) =>
      detail ? api.updateEngagement(detail.engagement.id, input) : api.createEngagement(input),
    onSuccess: (engagement) => {
      void queryClient.invalidateQueries({ queryKey: ["engagements"] });
      void queryClient.invalidateQueries({ queryKey: ["dashboard"] });
      if (detail) {
        void queryClient.invalidateQueries({ queryKey: ["engagement", detail.engagement.id] });
      }
      onOpenChange(false);
      if (!detail) {
        navigate(`/engagements/${engagement.id}`);
      }
    },
    onError: (error) => setServerError(error.message),
  });

  return (
    <Dialog
      open={open}
      onOpenChange={(next) => {
        if (!next) setServerError(null);
        onOpenChange(next);
      }}
      title={editing ? "Edit engagement" : "New engagement"}
      description={
        editing
          ? "Update scope, risks, assumptions, and contacts. Changes are audited."
          : "Start with the engagement basics; scope and test cases can be added next."
      }
      wide
      footer={
        <>
          <Button variant="secondary" onClick={() => onOpenChange(false)}>
            Cancel
          </Button>
          <Button type="submit" form={formId} disabled={mutation.isPending}>
            {mutation.isPending && <Spinner />}
            {editing ? "Save changes" : "Create engagement"}
          </Button>
        </>
      }
    >
      <EngagementForm
        key={detail?.engagement.id ?? "new"}
        formId={formId}
        defaultValues={detail ? valuesFromDetail(detail) : emptyEngagementValues()}
        onSubmit={(input) => {
          setServerError(null);
          mutation.mutate(input);
        }}
        submitting={mutation.isPending}
        error={serverError}
      />
    </Dialog>
  );
}
