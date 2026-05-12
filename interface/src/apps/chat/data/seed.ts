import type { Conversation, Message } from "../types";

const NOW = Date.now();
const MIN = 60 * 1000;
const HOUR = 60 * MIN;
const DAY = 24 * HOUR;

function iso(offsetMs: number): string {
  return new Date(NOW - offsetMs).toISOString();
}

export const SEED_CONVERSATIONS: Conversation[] = [
  {
    id: "maia",
    name: "Maia",
    roleBadge: "CEO",
    lastSnippet: "Strategic, efficient, and proactive...",
    updatedAt: iso(3 * HOUR + 25 * MIN),
  },
  {
    id: "test",
    name: "Test",
    roleBadge: "MEOW",
    lastSnippet: "Meow",
    updatedAt: iso(3 * DAY),
  },
  {
    id: "silo",
    name: "Silo",
    roleBadge: "CTO",
    lastSnippet: "Super-genius",
    updatedAt: iso(6 * DAY),
  },
  {
    id: "glenn",
    name: "Glenn",
    roleBadge: "CRO",
    lastSnippet: "CRO",
    updatedAt: iso(18 * DAY),
  },
  {
    id: "machina",
    name: "Machina",
    roleBadge: "DEVOPS",
    lastSnippet: "DevOps",
    updatedAt: iso(18 * DAY + 1 * HOUR),
  },
  {
    id: "nietzsche",
    name: "Nietzsche",
    roleBadge: "WRITER",
    lastSnippet: "Writer",
    updatedAt: iso(18 * DAY + 2 * HOUR),
  },
  {
    id: "yang",
    name: "Yang",
    roleBadge: "QUANT",
    lastSnippet: "Quant",
    updatedAt: iso(18 * DAY + 3 * HOUR),
  },
  {
    id: "elliot",
    name: "Elliot",
    roleBadge: "OPSEC",
    lastSnippet: "OpSec",
    updatedAt: iso(18 * DAY + 4 * HOUR),
  },
  {
    id: "gordon",
    name: "Gordon",
    roleBadge: "CFO",
    lastSnippet: "CFO",
    updatedAt: iso(22 * DAY),
  },
  {
    id: "ben",
    name: "Ben",
    roleBadge: "ACCOUNTANT",
    lastSnippet: "Accountant",
    updatedAt: iso(27 * DAY),
  },
  {
    id: "barret",
    name: "Barret",
    roleBadge: "GROWTH HACKER",
    lastSnippet: "Growth Hacker",
    updatedAt: iso(36 * DAY),
  },
  {
    id: "logos",
    name: "Logos",
    roleBadge: "RESEARCH",
    lastSnippet: "Research",
    updatedAt: iso(39 * DAY),
  },
  {
    id: "don-clawd",
    name: "Don Clawd",
    roleBadge: "CMO",
    lastSnippet: "Marketing and brand strategy",
    updatedAt: iso(39 * DAY + 4 * HOUR),
  },
];

export const SEED_MESSAGES: Message[] = [
  {
    id: "maia-1",
    conversationId: "maia",
    role: "contact",
    content: "Morning. Want me to summarize last week's milestones?",
    createdAt: iso(3 * HOUR + 40 * MIN),
  },
  {
    id: "maia-2",
    conversationId: "maia",
    role: "user",
    content: "Yes, focus on shipped work and what's blocked.",
    createdAt: iso(3 * HOUR + 32 * MIN),
  },
  {
    id: "maia-3",
    conversationId: "maia",
    role: "contact",
    content: "Strategic, efficient, and proactive — three shipped, one blocked on review.",
    createdAt: iso(3 * HOUR + 25 * MIN),
  },
  {
    id: "silo-1",
    conversationId: "silo",
    role: "contact",
    content: "Super-genius mode engaged. What architecture call are we debating?",
    createdAt: iso(6 * DAY),
  },
  {
    id: "test-1",
    conversationId: "test",
    role: "contact",
    content: "Meow",
    createdAt: iso(3 * DAY),
  },
];
