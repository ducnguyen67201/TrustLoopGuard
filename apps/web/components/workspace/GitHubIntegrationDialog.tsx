'use client';

import { IconBrandGithub, IconExternalLink, IconRefresh, IconShieldCheck } from '@tabler/icons-react';
import { useEffect, useMemo, useState, type ReactNode } from 'react';
import { toast } from 'sonner';

import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { Checkbox } from '@/components/ui/checkbox';
import { Dialog, DialogContent, DialogFooter, DialogTrigger } from '@/components/ui/dialog';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { Textarea } from '@/components/ui/textarea';
import {
  approveJob,
  createConnection,
  createInstallUrl,
  createJob,
  getJob,
  listConnections,
  listRepositories,
  type GitHubConnection,
  type GitHubIntegrationJob,
  type GitHubRepository,
} from '@/lib/github-integration';
import {
  DialogShellHeader,
  FieldHint,
  FormRow,
} from '@/components/workspace/dialog-scaffold';

type Props = {
  agentId: string;
  agentName: string;
  environmentId: string;
  children: ReactNode;
};

const terminalStatuses = new Set(['awaiting_approval', 'draft_pr_open', 'verified', 'error', 'cancelled', 'closed_unmerged']);

export function GitHubIntegrationDialog({ agentId, agentName, environmentId, children }: Props) {
  const [open, setOpen] = useState(false);
  const [repositories, setRepositories] = useState<GitHubRepository[]>([]);
  const [connections, setConnections] = useState<GitHubConnection[]>([]);
  const [repositoryId, setRepositoryId] = useState('');
  const [rootPath, setRootPath] = useState('');
  const [riskStatement, setRiskStatement] = useState('');
  const [consent, setConsent] = useState(false);
  const [job, setJob] = useState<GitHubIntegrationJob | null>(null);
  const [loading, setLoading] = useState(false);

  const selectedRepository = repositories.find((repo) => repo.repository_id === repositoryId);
  const selectedConnection = useMemo(
    () =>
      connections.find(
        (connection) =>
          connection.repository_id === repositoryId &&
          connection.root_path === rootPath.trim() &&
          connection.environment_id === environmentId,
      ),
    [connections, environmentId, repositoryId, rootPath],
  );

  useEffect(() => {
    if (!open) return;
    void refresh();
  }, [open]);

  useEffect(() => {
    if (job === null || terminalStatuses.has(job.status)) return;
    const controller = new AbortController();
    const timer = window.setInterval(async () => {
      try {
        setJob(await getJob(job.id, controller.signal));
      } catch {
        window.clearInterval(timer);
      }
    }, 4000);
    return () => {
      controller.abort();
      window.clearInterval(timer);
    };
  }, [job]);

  async function refresh() {
    setLoading(true);
    try {
      const [repos, currentConnections] = await Promise.all([
        listRepositories().catch(() => []),
        listConnections(agentId).catch(() => []),
      ]);
      setRepositories(repos);
      setConnections(currentConnections);
      setRepositoryId((current) => current || repos[0]?.repository_id || '');
    } finally {
      setLoading(false);
    }
  }

  async function install() {
    setLoading(true);
    try {
      window.location.href = await createInstallUrl();
    } catch (error) {
      toast.error(error instanceof Error ? error.message : 'Could not start GitHub install');
      setLoading(false);
    }
  }

  async function analyze() {
    if (repositoryId === '' || riskStatement.trim().length < 20 || !consent) return;
    setLoading(true);
    try {
      const connection =
        selectedConnection ??
        (await createConnection({
          repositoryId,
          rootPath: rootPath.trim(),
          agentId,
          environmentId,
        }));
      const nextJob = await createJob({
        connectionId: connection.id,
        riskStatement: riskStatement.trim(),
        sourceProcessingConsent: consent,
      });
      setJob(nextJob);
      setConnections((items) =>
        items.some((item) => item.id === connection.id) ? items : [connection, ...items],
      );
    } catch (error) {
      toast.error(error instanceof Error ? error.message : 'Could not start analysis');
    } finally {
      setLoading(false);
    }
  }

  async function approve() {
    if (job === null) return;
    setLoading(true);
    try {
      setJob(await approveJob(job.id));
      toast.success('Draft PR requested');
    } catch (error) {
      toast.error(error instanceof Error ? error.message : 'Could not open draft PR');
    } finally {
      setLoading(false);
    }
  }

  const canAnalyze =
    repositoryId !== '' && riskStatement.trim().length >= 20 && consent && !loading;

  return (
    <Dialog open={open} onOpenChange={setOpen}>
      <DialogTrigger asChild>{children}</DialogTrigger>
      <DialogContent className="max-h-[92vh] overflow-y-auto sm:max-w-3xl">
        <DialogShellHeader
          icon={<IconBrandGithub />}
          eyebrow="Assisted install"
          title="Connect GitHub"
          description={`Open a draft TrustLoopGuard integration PR for ${agentName}.`}
        />

        <div className="grid gap-5">
          <div className="flex items-center justify-between gap-3 rounded-lg border p-3">
            <div>
              <p className="text-sm font-medium">GitHub App</p>
              <p className="text-xs text-muted-foreground">
                Install on selected repositories only.
              </p>
            </div>
            <Button type="button" variant="outline" onClick={install} disabled={loading}>
              <IconBrandGithub />
              Install
            </Button>
          </div>

          <FormRow>
            <Label htmlFor="github-repo">Repository</Label>
            <select
              id="github-repo"
              className="h-10 rounded-md border bg-background px-3 text-sm"
              value={repositoryId}
              onChange={(event) => setRepositoryId(event.target.value)}
            >
              {repositories.length === 0 ? (
                <option value="">No repositories connected</option>
              ) : (
                repositories.map((repo) => (
                  <option key={repo.repository_id} value={repo.repository_id}>
                    {repo.full_name}
                  </option>
                ))
              )}
            </select>
            <FieldHint>
              {selectedRepository?.archived ? 'Archived repositories are not recommended.' : 'Root defaults to the repository root.'}
            </FieldHint>
          </FormRow>

          <FormRow>
            <Label htmlFor="github-root">Root path</Label>
            <Input
              id="github-root"
              value={rootPath}
              onChange={(event) => setRootPath(event.target.value)}
              placeholder="apps/web"
            />
          </FormRow>

          <FormRow>
            <Label htmlFor="github-risk">Irreversible action to guard</Label>
            <Textarea
              id="github-risk"
              value={riskStatement}
              onChange={(event) => setRiskStatement(event.target.value)}
              placeholder="Never send money or call a write API without an approved mandate."
              rows={4}
            />
          </FormRow>

          <label className="flex items-start gap-3 rounded-lg border p-3 text-sm">
            <Checkbox checked={consent} onCheckedChange={(value) => setConsent(value === true)} />
            <span>
              I understand selected source excerpts are sent to the configured control-plane LLM
              to generate a reviewable plan.
            </span>
          </label>

          {job !== null && <JobReview job={job} />}
        </div>

        <DialogFooter className="mt-1 gap-2 border-t pt-4">
          <Button type="button" variant="outline" onClick={refresh} disabled={loading}>
            <IconRefresh />
            Refresh
          </Button>
          {job?.status === 'awaiting_approval' ? (
            <Button type="button" onClick={approve} disabled={loading}>
              <IconShieldCheck />
              Open draft PR
            </Button>
          ) : (
            <Button type="button" onClick={analyze} disabled={!canAnalyze}>
              Analyze
            </Button>
          )}
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}

function JobReview({ job }: { job: GitHubIntegrationJob }) {
  return (
    <div className="rounded-lg border p-4">
      <div className="flex items-center justify-between gap-3">
        <div>
          <p className="text-sm font-medium">Analysis</p>
          <p className="text-xs text-muted-foreground">
            {job.analysis_summary?.summary ?? 'Analysis is running.'}
          </p>
        </div>
        <Badge variant="outline">{job.status.replaceAll('_', ' ')}</Badge>
      </div>
      {job.error_message !== undefined && job.error_message !== null ? (
        <p className="mt-3 text-sm text-destructive">{job.error_message}</p>
      ) : null}
      {job.proposed_changes.length > 0 ? (
        <div className="mt-4 grid gap-3">
          {job.proposed_changes.map((change) => (
            <div key={change.path} className="rounded-md border bg-muted/30 p-3">
              <div className="flex items-center justify-between gap-3">
                <code className="text-xs">{change.path}</code>
                <Badge>{change.operation}</Badge>
              </div>
              <p className="mt-2 text-xs text-muted-foreground">{change.rationale}</p>
            </div>
          ))}
        </div>
      ) : null}
      {job.pull_request_url !== undefined && job.pull_request_url !== null ? (
        <Button asChild className="mt-4" variant="outline">
          <a href={job.pull_request_url} target="_blank" rel="noreferrer">
            <IconExternalLink />
            Open PR
          </a>
        </Button>
      ) : null}
    </div>
  );
}
