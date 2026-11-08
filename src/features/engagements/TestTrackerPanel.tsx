import { useEffect, useMemo, useState } from "react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { Controller, useForm } from "react-hook-form";
import { zodResolver } from "@hookform/resolvers/zod";
import { z } from "zod";
import { ArrowDown, ArrowUp, ListChecks, Pencil, Plus, Search, Trash2 } from "lucide-react";
import { Button } from "@/components/ui/button";
import { Card } from "@/components/ui/card";
import { Badge } from "@/components/ui/badge";
import { Dialog } from "@/components/ui/dialog";
import { Field, Input, Textarea } from "@/components/ui/input";
import { Select, SelectItem } from "@/components/ui/select";
import { EmptyState, ErrorBlock, LoadingBlock, Spinner } from "@/components/ui/states";
import { api } from "@/lib/api";
import { testCaseStatusLabel } from "@/lib/labels";
import type { TestCase, TestCaseInput, TestCaseStatus, TestCaseTemplate } from "@/types";

export function TestTrackerPanel({ engagementId }: { engagementId: string }) {
  const queryClient = useQueryClient();
  const [templatesOpen, setTemplatesOpen] = useState(false);
  const [editing, setEditing] = useState<TestCase | null>(null);
  const [customOpen, setCustomOpen] = useState(false);
  const [search, setSearch] = useState("");
  const [statusFilter, setStatusFilter] = useState<TestCaseStatus | "all">("all");
  const testCases = useQuery({
    queryKey: ["test-cases", engagementId],
    queryFn: () => api.listTestCases(engagementId),
  });

  const invalidate = () => {
    void queryClient.invalidateQueries({ queryKey: ["test-cases", engagementId] });
    void queryClient.invalidateQueries({ queryKey: ["engagement", engagementId] });
    void queryClient.invalidateQueries({ queryKey: ["engagements"] });
    void queryClient.invalidateQueries({ queryKey: ["dashboard"] });
  };

  const update = useMutation({
    mutationFn: ({ id, input }: { id: string; input: TestCaseInput }) =>
      api.updateTestCase(id, input),
    onSuccess: invalidate,
  });

  const remove = useMutation({
    mutationFn: (id: string) => api.deleteTestCase(id),
    onSuccess: invalidate,
  });

  const reorder = useMutation({
    mutationFn: (orderedIds: string[]) => api.reorderTestCases(engagementId, orderedIds),
    onSuccess: invalidate,
  });

  const filtered = useMemo(() => {
    const query = search.trim().toLowerCase();
    return (testCases.data ?? [])
      .map((testCase, index) => ({ testCase, index }))
      .filter(({ testCase }) => {
        const matchesStatus = statusFilter === "all" || testCase.status === statusFilter;
        const matchesSearch =
          !query ||
          testCase.title.toLowerCase().includes(query) ||
          testCase.category.toLowerCase().includes(query) ||
          testCase.approach.toLowerCase().includes(query);
        return matchesStatus && matchesSearch;
      });
  }, [testCases.data, search, statusFilter]);

  const canReorder = !search.trim() && statusFilter === "all";

  const move = (index: number, direction: -1 | 1) => {
    const data = testCases.data ?? [];
    const target = index + direction;
    if (target < 0 || target >= data.length) return;
    const ids = data.map((testCase) => testCase.id);
    [ids[index], ids[target]] = [ids[target], ids[index]];
    reorder.mutate(ids);
  };

  return (
    <div className="space-y-4">
      <div className="flex flex-wrap items-center justify-between gap-3">
        <div>
          <h2 className="text-sm font-semibold text-white">Test case tracker</h2>
          <p className="text-xs text-slate-500">
            {testCases.data?.length ?? 0} test cases · status, result, and evidence stay separate
          </p>
        </div>
        <div className="flex gap-2">
          <Button variant="secondary" size="sm" onClick={() => setTemplatesOpen(true)}>
            <ListChecks className="h-3.5 w-3.5" /> From templates
          </Button>
          <Button size="sm" onClick={() => setCustomOpen(true)}>
            <Plus className="h-3.5 w-3.5" /> Custom test
          </Button>
        </div>
      </div>

      <div className="flex flex-wrap items-center gap-2">
        <div className="relative min-w-0 flex-1 sm:max-w-sm">
          <Search className="pointer-events-none absolute left-3 top-1/2 h-4 w-4 -translate-y-1/2 text-slate-500" />
          <Input
            value={search}
            onChange={(event) => setSearch(event.target.value)}
            placeholder="Search title, category, or approach"
            className="pl-9"
          />
        </div>
        <Select
          value={statusFilter}
          onValueChange={(value) => setStatusFilter(value as TestCaseStatus | "all")}
          className="w-44"
        >
          <SelectItem value="all">All statuses</SelectItem>
          {(Object.keys(testCaseStatusLabel) as TestCaseStatus[]).map((status) => (
            <SelectItem key={status} value={status}>
              {testCaseStatusLabel[status]}
            </SelectItem>
          ))}
        </Select>
        {!canReorder && (
          <Button
            variant="ghost"
            size="sm"
            onClick={() => {
              setSearch("");
              setStatusFilter("all");
            }}
          >
            Clear filters
          </Button>
        )}
      </div>

      {testCases.isLoading && <LoadingBlock label="Loading test cases…" />}
      {testCases.isError && <ErrorBlock message={testCases.error.message} />}
      {(update.isError || remove.isError || reorder.isError) && (
        <ErrorBlock message={(update.error ?? remove.error ?? reorder.error)!.message} />
      )}

      {testCases.data && testCases.data.length === 0 && (
        <EmptyState
          title="No test cases yet"
          description="Add reusable test cases from the built-in domain templates or create a custom test."
          action={
            <Button size="sm" onClick={() => setTemplatesOpen(true)}>
              <ListChecks className="h-3.5 w-3.5" /> Choose templates
            </Button>
          }
        />
      )}

      {testCases.data && testCases.data.length > 0 && filtered.length === 0 && (
        <EmptyState
          title="No test cases match"
          description="Try a different search term or status filter."
          action={
            <Button
              size="sm"
              variant="secondary"
              onClick={() => {
                setSearch("");
                setStatusFilter("all");
              }}
            >
              Clear filters
            </Button>
          }
        />
      )}

      {filtered.length > 0 && (
        <Card className="overflow-hidden">
          <div className="overflow-x-auto">
            <table className="w-full text-left text-sm">
              <thead className="border-b border-slate-800 bg-slate-950/40 text-[11px] uppercase tracking-wide text-slate-500">
                <tr>
                  <th className="px-4 py-3 font-medium">Test</th>
                  <th className="px-4 py-3 font-medium">Status</th>
                  <th className="px-4 py-3 font-medium">Result</th>
                  <th className="px-4 py-3 text-right font-medium">Actions</th>
                </tr>
              </thead>
              <tbody className="divide-y divide-slate-800">
                {filtered.map(({ testCase, index }) => (
                  <tr
                    key={testCase.id}
                    className="align-top transition-colors hover:bg-slate-800/20"
                  >
                    <td className="max-w-md px-4 py-3">
                      <p className="font-medium text-slate-100">{testCase.title}</p>
                      {testCase.category && (
                        <p className="mt-0.5 text-xs text-slate-500">{testCase.category}</p>
                      )}
                      {testCase.approach && (
                        <p className="mt-1 line-clamp-2 text-xs leading-relaxed text-slate-500">
                          {testCase.approach}
                        </p>
                      )}
                    </td>
                    <td className="px-4 py-3">
                      <Select
                        className="h-8 w-36 text-xs"
                        value={testCase.status}
                        onValueChange={(value) =>
                          update.mutate({
                            id: testCase.id,
                            input: toInput(testCase, {
                              status: value as TestCaseStatus,
                            }),
                          })
                        }
                      >
                        {(Object.keys(testCaseStatusLabel) as TestCaseStatus[]).map((status) => (
                          <SelectItem key={status} value={status}>
                            {testCaseStatusLabel[status]}
                          </SelectItem>
                        ))}
                      </Select>
                    </td>
                    <td className="max-w-xs px-4 py-3 text-xs text-slate-400">
                      {testCase.result ? (
                        <span className="line-clamp-3">{testCase.result}</span>
                      ) : (
                        <span className="text-slate-600">No result recorded</span>
                      )}
                    </td>
                    <td className="px-4 py-3">
                      <div className="flex justify-end gap-1">
                        <Button
                          variant="ghost"
                          size="icon"
                          aria-label="Move test case up"
                          disabled={!canReorder || index === 0 || reorder.isPending}
                          onClick={() => move(index, -1)}
                        >
                          <ArrowUp className="h-3.5 w-3.5" />
                        </Button>
                        <Button
                          variant="ghost"
                          size="icon"
                          aria-label="Move test case down"
                          disabled={
                            !canReorder ||
                            index === (testCases.data?.length ?? 1) - 1 ||
                            reorder.isPending
                          }
                          onClick={() => move(index, 1)}
                        >
                          <ArrowDown className="h-3.5 w-3.5" />
                        </Button>
                        <Button
                          variant="ghost"
                          size="icon"
                          aria-label="Edit test case"
                          onClick={() => setEditing(testCase)}
                        >
                          <Pencil className="h-3.5 w-3.5" />
                        </Button>
                        <Button
                          variant="ghost"
                          size="icon"
                          aria-label="Delete test case"
                          onClick={() => {
                            if (window.confirm(`Delete "${testCase.title}"?`)) {
                              remove.mutate(testCase.id);
                            }
                          }}
                        >
                          <Trash2 className="h-3.5 w-3.5 text-slate-500" />
                        </Button>
                      </div>
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        </Card>
      )}

      <TemplatePickerDialog
        open={templatesOpen}
        onOpenChange={setTemplatesOpen}
        engagementId={engagementId}
        onCreated={invalidate}
      />
      {customOpen && (
        <TestCaseDialog
          open={customOpen}
          onOpenChange={setCustomOpen}
          engagementId={engagementId}
          testCase={null}
          defaultSortOrder={testCases.data?.length ?? 0}
          onSaved={invalidate}
        />
      )}
      {editing && (
        <TestCaseDialog
          key={editing.id}
          open
          onOpenChange={(open) => !open && setEditing(null)}
          engagementId={engagementId}
          testCase={editing}
          defaultSortOrder={editing.sortOrder}
          onSaved={invalidate}
        />
      )}
    </div>
  );
}

function toInput(testCase: TestCase, overrides: Partial<TestCaseInput> = {}): TestCaseInput {
  return {
    engagementId: testCase.engagementId,
    templateId: testCase.templateId,
    title: testCase.title,
    approach: testCase.approach,
    category: testCase.category,
    status: testCase.status,
    result: testCase.result,
    notes: testCase.notes,
    assignee: testCase.assignee,
    sortOrder: testCase.sortOrder,
    ...overrides,
  };
}

const testCaseSchema = z.object({
  title: z.string().trim().min(1, "Enter a test title.").max(300),
  category: z.string().max(120),
  status: z.enum(["not_started", "in_progress", "done", "blocked", "not_applicable"]),
  approach: z.string().max(20_000),
  result: z.string().max(20_000),
  notes: z.string().max(20_000),
});

type TestCaseValues = z.infer<typeof testCaseSchema>;

function TestCaseDialog({
  open,
  onOpenChange,
  engagementId,
  testCase,
  defaultSortOrder,
  onSaved,
}: {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  engagementId: string;
  testCase: TestCase | null;
  defaultSortOrder: number;
  onSaved: () => void;
}) {
  const [serverError, setServerError] = useState<string | null>(null);
  const form = useForm<TestCaseValues>({
    resolver: zodResolver(testCaseSchema),
    defaultValues: valuesFor(testCase),
  });
  useEffect(() => {
    if (open) {
      form.reset(valuesFor(testCase));
    }
  }, [open, testCase, form]);
  const mutation = useMutation({
    mutationFn: (values: TestCaseValues) => {
      const input: TestCaseInput = {
        engagementId,
        templateId: testCase?.templateId ?? null,
        title: values.title.trim(),
        approach: values.approach,
        category: values.category.trim(),
        status: values.status,
        result: values.result,
        notes: values.notes,
        assignee: testCase?.assignee ?? "",
        sortOrder: testCase?.sortOrder ?? defaultSortOrder,
      };
      return testCase ? api.updateTestCase(testCase.id, input) : api.createTestCase(input);
    },
    onSuccess: () => {
      onSaved();
      onOpenChange(false);
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
      title={testCase ? "Edit test case" : "New test case"}
      description="Record the approach, execution status, result, and notes."
      wide
      footer={
        <>
          <Button variant="secondary" onClick={() => onOpenChange(false)}>
            Cancel
          </Button>
          <Button type="submit" form="test-case-form" disabled={mutation.isPending}>
            {mutation.isPending && <Spinner />}
            Save test case
          </Button>
        </>
      }
    >
      <form
        id="test-case-form"
        className="space-y-4"
        onSubmit={form.handleSubmit((values) => {
          setServerError(null);
          mutation.mutate(values);
        })}
      >
        <div className="grid gap-4 sm:grid-cols-[1fr_220px]">
          <Field label="Title" error={form.formState.errors.title?.message}>
            <Input autoFocus {...form.register("title")} />
          </Field>
          <Field label="Status">
            <Controller
              control={form.control}
              name="status"
              render={({ field }) => (
                <Select value={field.value} onValueChange={field.onChange}>
                  {(Object.keys(testCaseStatusLabel) as TestCaseStatus[]).map((status) => (
                    <SelectItem key={status} value={status}>
                      {testCaseStatusLabel[status]}
                    </SelectItem>
                  ))}
                </Select>
              )}
            />
          </Field>
        </div>
        <Field label="Category">
          <Input placeholder="Injection, authentication…" {...form.register("category")} />
        </Field>
        <Field label="Test approach">
          <Textarea rows={3} {...form.register("approach")} />
        </Field>
        <Field label="Test result">
          <Textarea rows={3} {...form.register("result")} />
        </Field>
        <Field label="Notes">
          <Textarea rows={2} {...form.register("notes")} />
        </Field>
        {serverError && (
          <p className="rounded-lg border border-red-500/30 bg-red-500/5 px-3 py-2 text-xs text-red-300">
            {serverError}
          </p>
        )}
        <button type="submit" className="hidden" />
      </form>
    </Dialog>
  );
}

function valuesFor(testCase: TestCase | null): TestCaseValues {
  return {
    title: testCase?.title ?? "",
    category: testCase?.category ?? "",
    status: testCase?.status ?? "not_started",
    approach: testCase?.approach ?? "",
    result: testCase?.result ?? "",
    notes: testCase?.notes ?? "",
  };
}

function TemplatePickerDialog({
  open,
  onOpenChange,
  engagementId,
  onCreated,
}: {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  engagementId: string;
  onCreated: () => void;
}) {
  const [selected, setSelected] = useState<Set<string>>(new Set());
  const [search, setSearch] = useState("");
  const templates = useQuery({
    queryKey: ["templates"],
    queryFn: () => api.listTestCaseTemplates(),
    enabled: open,
  });
  const create = useMutation({
    mutationFn: () => api.createTestCasesFromTemplates(engagementId, Array.from(selected)),
    onSuccess: () => {
      setSelected(new Set());
      setSearch("");
      onCreated();
      onOpenChange(false);
    },
  });

  const grouped = useMemo(() => {
    const query = search.trim().toLowerCase();
    const map = new Map<string, TestCaseTemplate[]>();
    for (const template of templates.data ?? []) {
      if (
        query &&
        !template.title.toLowerCase().includes(query) &&
        !template.category.toLowerCase().includes(query) &&
        !template.domain.toLowerCase().includes(query)
      ) {
        continue;
      }
      const list = map.get(template.domain) ?? [];
      list.push(template);
      map.set(template.domain, list);
    }
    return Array.from(map.entries());
  }, [templates.data, search]);

  const toggle = (id: string) => {
    setSelected((current) => {
      const next = new Set(current);
      if (next.has(id)) next.delete(id);
      else next.add(id);
      return next;
    });
  };

  return (
    <Dialog
      open={open}
      onOpenChange={(next) => {
        if (!next) setSearch("");
        onOpenChange(next);
      }}
      title="Add test cases from templates"
      description="Templates are reusable, domain-specific starting points. Customize them after adding."
      wide
      footer={
        <>
          <Button variant="secondary" onClick={() => onOpenChange(false)}>
            Cancel
          </Button>
          <Button
            onClick={() => create.mutate()}
            disabled={selected.size === 0 || create.isPending}
          >
            {create.isPending && <Spinner />}
            Add {selected.size || ""} test{selected.size === 1 ? "" : "s"}
          </Button>
        </>
      }
    >
      <div className="relative mb-4">
        <Search className="pointer-events-none absolute left-3 top-1/2 h-4 w-4 -translate-y-1/2 text-slate-500" />
        <Input
          value={search}
          onChange={(event) => setSearch(event.target.value)}
          placeholder="Search templates by domain, category, or title"
          className="pl-9"
        />
      </div>

      {templates.isLoading && <LoadingBlock label="Loading templates…" />}
      {templates.isError && <ErrorBlock message={templates.error.message} />}
      {create.isError && <ErrorBlock message={create.error.message} />}

      <div className="space-y-5">
        {grouped.map(([domain, items]) => (
          <div key={domain}>
            <div className="mb-2 flex items-center gap-2">
              <Badge tone="accent">{domain.replace("_", " ")}</Badge>
              <span className="text-xs text-slate-500">{items.length} templates</span>
            </div>
            <div className="grid gap-2 sm:grid-cols-2">
              {items.map((template) => (
                <label
                  key={template.id}
                  className="flex cursor-pointer items-start gap-3 rounded-lg border border-slate-800 bg-slate-900/40 p-3 transition-colors hover:border-slate-700"
                >
                  <input
                    type="checkbox"
                    className="mt-0.5 h-4 w-4 rounded border-slate-600 bg-slate-900"
                    checked={selected.has(template.id)}
                    onChange={() => toggle(template.id)}
                  />
                  <span>
                    <span className="block text-xs font-medium text-slate-200">
                      {template.title}
                    </span>
                    <span className="mt-0.5 block text-[11px] text-slate-500">
                      {template.category}
                      {template.references ? ` · ${template.references}` : ""}
                    </span>
                  </span>
                </label>
              ))}
            </div>
          </div>
        ))}
        {grouped.length === 0 && !templates.isLoading && (
          <p className="py-8 text-center text-xs text-slate-500">No templates match.</p>
        )}
      </div>
    </Dialog>
  );
}
