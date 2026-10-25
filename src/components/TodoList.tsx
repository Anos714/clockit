import {
  useCallback,
  useEffect,
  useRef,
  useState,
  type FormEvent,
  type KeyboardEvent,
} from "react";
import { AnimatePresence, motion } from "motion/react";
import { api } from "../api";
import type { Todo } from "../types";
import { CornerSprig, LeafGlyph } from "./Botanical";
import {
  CheckIcon,
  CloseIcon,
  PencilIcon,
  PlusIcon,
  TrashIcon,
} from "./Icons";

export function TodoList() {
  const [todos, setTodos] = useState<Todo[] | null>(null);
  const [text, setText] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const [editingId, setEditingId] = useState<number | null>(null);
  const [draft, setDraft] = useState("");
  const inputRef = useRef<HTMLInputElement>(null);
  const editRef = useRef<HTMLInputElement>(null);

  const refresh = useCallback(async () => {
    try {
      setTodos(await api.todos());
      setError(null);
    } catch (reason) {
      setError(String(reason));
    }
  }, []);

  useEffect(() => {
    void refresh();
  }, [refresh]);

  useEffect(() => {
    if (editingId !== null) editRef.current?.focus();
  }, [editingId]);

  const mutate = async (action: () => Promise<unknown>) => {
    setError(null);
    try {
      await action();
      await refresh();
    } catch (reason) {
      setError(String(reason));
    }
  };

  const add = async (event: FormEvent) => {
    event.preventDefault();
    const value = text.trim();
    if (!value || busy) return;
    setBusy(true);
    setError(null);
    try {
      await api.addTodo(value);
      setText("");
      await refresh();
      inputRef.current?.focus();
    } catch (reason) {
      setError(String(reason));
    } finally {
      setBusy(false);
    }
  };

  const cancelEdit = () => {
    setEditingId(null);
    setDraft("");
  };

  const startEdit = (todo: Todo) => {
    setEditingId(todo.id);
    setDraft(todo.text);
  };

  const saveEdit = async (todo: Todo) => {
    const value = draft.trim();
    if (!value) {
      setError("Task cannot be empty");
      return;
    }
    if (value === todo.text) {
      cancelEdit();
      return;
    }
    setError(null);
    try {
      await api.updateTodoText(todo.id, value);
      cancelEdit();
      await refresh();
    } catch (reason) {
      setError(String(reason));
    }
  };

  const onEditKeyDown = (
    event: KeyboardEvent<HTMLInputElement>,
    todo: Todo,
  ) => {
    if (event.key === "Enter") {
      event.preventDefault();
      void saveEdit(todo);
    } else if (event.key === "Escape") {
      event.preventDefault();
      cancelEdit();
    }
  };

  const doneCount = todos?.filter((todo) => todo.done).length ?? 0;
  const total = todos?.length ?? 0;
  const doneRatio = total > 0 ? doneCount / total : 0;

  return (
    <section className="card todo-card">
      <CornerSprig />
      <header className="card-head">
        <div>
          <span className="card-eyebrow">Next steps</span>
          <h2>Quick list</h2>
        </div>
        <span className="count" title={`${doneCount} of ${total} tasks done`}>
          {doneCount}/{total}
        </span>
      </header>

      {total > 0 ? (
        <div
          className="todo-meter"
          role="progressbar"
          aria-valuemin={0}
          aria-valuemax={total}
          aria-valuenow={doneCount}
          aria-label="Tasks completed"
        >
          <motion.span
            initial={false}
            animate={{ scaleX: doneRatio }}
            transition={{ type: "spring", stiffness: 180, damping: 26 }}
          />
        </div>
      ) : null}

      <form className="todo-form" onSubmit={add}>
        <input
          ref={inputRef}
          type="text"
          value={text}
          maxLength={280}
          placeholder="Add a small next step…"
          aria-label="New task"
          disabled={busy}
          onChange={(event) => setText(event.currentTarget.value)}
        />
        <motion.button
          className="todo-add"
          type="submit"
          title="Add task"
          aria-label="Add task"
          disabled={!text.trim() || busy}
          whileTap={{ scale: 0.92 }}
        >
          <PlusIcon size={17} />
        </motion.button>
      </form>

      {error ? (
        <p className="todo-error" role="alert">
          {error}
        </p>
      ) : null}

      {todos === null ? (
        <p className="empty">Gathering your list…</p>
      ) : todos.length === 0 ? (
        <div className="empty-state">
          <LeafGlyph size={22} />
          <p>Nothing queued. Add one small thing to grow into.</p>
        </div>
      ) : (
        <ul className="todo-list">
          <AnimatePresence initial={false}>
            {todos.map((todo) => {
              const editing = editingId === todo.id;
              return (
                <motion.li
                  key={todo.id}
                  layout="position"
                  initial={{ opacity: 0, y: -5 }}
                  animate={{ opacity: 1, y: 0 }}
                  exit={{ opacity: 0, x: 10 }}
                  transition={{ duration: 0.22, ease: [0.22, 1, 0.36, 1] }}
                  className={[
                    todo.done ? "done" : "",
                    editing ? "editing" : "",
                  ]
                    .filter(Boolean)
                    .join(" ")}
                >
                  <motion.button
                    type="button"
                    className="todo-check"
                    role="checkbox"
                    aria-checked={todo.done}
                    aria-label={
                      todo.done
                        ? `Mark ${todo.text} as not done`
                        : `Mark ${todo.text} as done`
                    }
                    disabled={editing}
                    whileTap={{ scale: 0.85 }}
                    onClick={() =>
                      void mutate(() => api.setTodoDone(todo.id, !todo.done))
                    }
                  >
                    {todo.done ? (
                      <motion.span
                        initial={{ scale: 0, rotate: -25 }}
                        animate={{ scale: 1, rotate: 0 }}
                        transition={{
                          type: "spring",
                          stiffness: 600,
                          damping: 24,
                        }}
                      >
                        <CheckIcon size={12} />
                      </motion.span>
                    ) : null}
                  </motion.button>

                  {editing ? (
                    <input
                      ref={editRef}
                      className="todo-edit"
                      value={draft}
                      maxLength={280}
                      aria-label="Edit task"
                      onChange={(event) => setDraft(event.currentTarget.value)}
                      onKeyDown={(event) => onEditKeyDown(event, todo)}
                    />
                  ) : (
                    <span
                      className="todo-text"
                      title="Double-click to edit"
                      onDoubleClick={() => startEdit(todo)}
                    >
                      {todo.text}
                    </span>
                  )}

                  <div className="todo-actions">
                    {editing ? (
                      <>
                        <button
                          type="button"
                          className="todo-action"
                          title="Save (Enter)"
                          aria-label="Save task"
                          onClick={() => void saveEdit(todo)}
                        >
                          <CheckIcon size={14} />
                        </button>
                        <button
                          type="button"
                          className="todo-action"
                          title="Cancel (Esc)"
                          aria-label="Cancel edit"
                          onClick={cancelEdit}
                        >
                          <CloseIcon size={14} />
                        </button>
                      </>
                    ) : (
                      <>
                        <button
                          type="button"
                          className="todo-action"
                          title="Edit task"
                          aria-label={`Edit ${todo.text}`}
                          onClick={() => startEdit(todo)}
                        >
                          <PencilIcon size={14} />
                        </button>
                        <button
                          type="button"
                          className="todo-action danger"
                          title="Delete task"
                          aria-label={`Delete ${todo.text}`}
                          onClick={() =>
                            void mutate(() => api.deleteTodo(todo.id))
                          }
                        >
                          <TrashIcon size={14} />
                        </button>
                      </>
                    )}
                  </div>
                </motion.li>
              );
            })}
          </AnimatePresence>
        </ul>
      )}
    </section>
  );
}
