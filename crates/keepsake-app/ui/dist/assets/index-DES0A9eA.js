var __defProp = Object.defineProperty;
var __defNormalProp = (obj, key, value) => key in obj ? __defProp(obj, key, { enumerable: true, configurable: true, writable: true, value }) : obj[key] = value;
var __publicField = (obj, key, value) => __defNormalProp(obj, typeof key !== "symbol" ? key + "" : key, value);
var _a;
(function polyfill() {
  const relList = document.createElement("link").relList;
  if (relList && relList.supports && relList.supports("modulepreload")) {
    return;
  }
  for (const link of document.querySelectorAll('link[rel="modulepreload"]')) {
    processPreload(link);
  }
  new MutationObserver((mutations) => {
    for (const mutation of mutations) {
      if (mutation.type !== "childList") {
        continue;
      }
      for (const node of mutation.addedNodes) {
        if (node.tagName === "LINK" && node.rel === "modulepreload")
          processPreload(node);
      }
    }
  }).observe(document, { childList: true, subtree: true });
  function getFetchOpts(link) {
    const fetchOpts = {};
    if (link.integrity) fetchOpts.integrity = link.integrity;
    if (link.referrerPolicy) fetchOpts.referrerPolicy = link.referrerPolicy;
    if (link.crossOrigin === "use-credentials")
      fetchOpts.credentials = "include";
    else if (link.crossOrigin === "anonymous") fetchOpts.credentials = "omit";
    else fetchOpts.credentials = "same-origin";
    return fetchOpts;
  }
  function processPreload(link) {
    if (link.ep)
      return;
    link.ep = true;
    const fetchOpts = getFetchOpts(link);
    fetch(link.href, fetchOpts);
  }
})();
const IS_DEV = false;
const equalFn = (a, b2) => a === b2;
const $PROXY = Symbol("solid-proxy");
const SUPPORTS_PROXY = typeof Proxy === "function";
const $TRACK = Symbol("solid-track");
const signalOptions = {
  equals: equalFn
};
let runEffects = runQueue;
const STALE = 1;
const PENDING = 2;
const UNOWNED = {
  owned: null,
  cleanups: null,
  context: null,
  owner: null
};
const NO_INIT = {};
var Owner = null;
let Transition = null;
let ExternalSourceConfig = null;
let Listener = null;
let Updates = null;
let Effects = null;
let ExecCount = 0;
function createRoot(fn, detachedOwner) {
  const listener = Listener, owner = Owner, unowned = fn.length === 0, current = detachedOwner === void 0 ? owner : detachedOwner, root2 = unowned ? UNOWNED : {
    owned: null,
    cleanups: null,
    context: current ? current.context : null,
    owner: current
  }, updateFn = unowned ? fn : () => fn(() => untrack(() => cleanNode(root2)));
  Owner = root2;
  Listener = null;
  try {
    return runUpdates(updateFn, true);
  } finally {
    Listener = listener;
    Owner = owner;
  }
}
function createSignal(value, options) {
  options = options ? Object.assign({}, signalOptions, options) : signalOptions;
  const s = {
    value,
    observers: null,
    observerSlots: null,
    comparator: options.equals || void 0
  };
  const setter = (value2) => {
    if (typeof value2 === "function") {
      value2 = value2(s.value);
    }
    return writeSignal(s, value2);
  };
  return [readSignal.bind(s), setter];
}
function createComputed(fn, value, options) {
  const c = createComputation(fn, value, true, STALE);
  updateComputation(c);
}
function createRenderEffect(fn, value, options) {
  const c = createComputation(fn, value, false, STALE);
  updateComputation(c);
}
function createEffect(fn, value, options) {
  runEffects = runUserEffects;
  const c = createComputation(fn, value, false, STALE);
  c.user = true;
  Effects ? Effects.push(c) : updateComputation(c);
}
function createMemo(fn, value, options) {
  options = options ? Object.assign({}, signalOptions, options) : signalOptions;
  const c = createComputation(fn, value, true, 0);
  c.observers = null;
  c.observerSlots = null;
  c.comparator = options.equals || void 0;
  updateComputation(c);
  return readSignal.bind(c);
}
function isPromise(v2) {
  return v2 && typeof v2 === "object" && "then" in v2;
}
function createResource(pSource, pFetcher, pOptions) {
  let source;
  let fetcher;
  let options;
  if (typeof pFetcher === "function") {
    source = pSource;
    fetcher = pFetcher;
    options = {};
  } else {
    source = true;
    fetcher = pSource;
    options = pFetcher || {};
  }
  let pr = null, initP = NO_INIT, scheduled = false, resolved = "initialValue" in options, dynamic = typeof source === "function" && createMemo(source);
  const contexts = /* @__PURE__ */ new Set(), [value, setValue] = (options.storage || createSignal)(options.initialValue), [error, setError] = createSignal(void 0), [track, trigger] = createSignal(void 0, {
    equals: false
  }), [state2, setState] = createSignal(resolved ? "ready" : "unresolved");
  function loadEnd(p, v2, error2, key) {
    if (pr === p) {
      pr = null;
      key !== void 0 && (resolved = true);
      if ((p === initP || v2 === initP) && options.onHydrated) queueMicrotask(() => options.onHydrated(key, {
        value: v2
      }));
      initP = NO_INIT;
      completeLoad(v2, error2);
    }
    return v2;
  }
  function completeLoad(v2, err) {
    runUpdates(() => {
      if (err === void 0) setValue(() => v2);
      setState(err !== void 0 ? "errored" : resolved ? "ready" : "unresolved");
      setError(err);
      for (const c of contexts.keys()) c.decrement();
      contexts.clear();
    }, false);
  }
  function read() {
    const c = SuspenseContext, v2 = value(), err = error();
    if (err !== void 0 && !pr) throw err;
    if (Listener && !Listener.user && c) ;
    return v2;
  }
  function load(refetching = true) {
    if (refetching !== false && scheduled) return;
    scheduled = false;
    const lookup = dynamic ? dynamic() : source;
    if (lookup == null || lookup === false) {
      loadEnd(pr, untrack(value));
      return;
    }
    let error2;
    const p = initP !== NO_INIT ? initP : untrack(() => {
      try {
        return fetcher(lookup, {
          value: value(),
          refetching
        });
      } catch (fetcherError) {
        error2 = fetcherError;
      }
    });
    if (error2 !== void 0) {
      loadEnd(pr, void 0, castError(error2), lookup);
      return;
    } else if (!isPromise(p)) {
      loadEnd(pr, p, void 0, lookup);
      return p;
    }
    pr = p;
    if ("v" in p) {
      if (p.s === 1) loadEnd(pr, p.v, void 0, lookup);
      else loadEnd(pr, void 0, castError(p.v), lookup);
      return p;
    }
    scheduled = true;
    queueMicrotask(() => scheduled = false);
    runUpdates(() => {
      setState(resolved ? "refreshing" : "pending");
      trigger();
    }, false);
    return p.then((v2) => loadEnd(p, v2, void 0, lookup), (e) => loadEnd(p, void 0, castError(e), lookup));
  }
  Object.defineProperties(read, {
    state: {
      get: () => state2()
    },
    error: {
      get: () => error()
    },
    loading: {
      get() {
        const s = state2();
        return s === "pending" || s === "refreshing";
      }
    },
    latest: {
      get() {
        if (!resolved) return read();
        const err = error();
        if (err && !pr) throw err;
        return value();
      }
    }
  });
  let owner = Owner;
  if (dynamic) createComputed(() => (owner = Owner, load(false)));
  else load(false);
  return [read, {
    refetch: (info) => runWithOwner(owner, () => load(info)),
    mutate: setValue
  }];
}
function batch(fn) {
  return runUpdates(fn, false);
}
function untrack(fn) {
  if (Listener === null) return fn();
  const listener = Listener;
  Listener = null;
  try {
    if (ExternalSourceConfig) ;
    return fn();
  } finally {
    Listener = listener;
  }
}
function on(deps, fn, options) {
  const isArray = Array.isArray(deps);
  let prevInput;
  let defer = options && options.defer;
  return (prevValue) => {
    let input;
    if (isArray) {
      input = Array(deps.length);
      for (let i = 0; i < deps.length; i++) input[i] = deps[i]();
    } else input = deps();
    if (defer) {
      defer = false;
      return prevValue;
    }
    const result = untrack(() => fn(input, prevInput, prevValue));
    prevInput = input;
    return result;
  };
}
function onMount(fn) {
  createEffect(() => untrack(fn));
}
function onCleanup(fn) {
  if (Owner === null) ;
  else if (Owner.cleanups === null) Owner.cleanups = [fn];
  else Owner.cleanups.push(fn);
  return fn;
}
function getOwner() {
  return Owner;
}
function runWithOwner(o, fn) {
  const prev = Owner;
  const prevListener = Listener;
  Owner = o;
  Listener = null;
  try {
    return runUpdates(fn, true);
  } catch (err) {
    handleError(err);
  } finally {
    Owner = prev;
    Listener = prevListener;
  }
}
function startTransition(fn) {
  const l3 = Listener;
  const o = Owner;
  return Promise.resolve().then(() => {
    Listener = l3;
    Owner = o;
    let t;
    runUpdates(fn, false);
    Listener = Owner = null;
    return t ? t.done : void 0;
  });
}
const [transPending, setTransPending] = /* @__PURE__ */ createSignal(false);
function createContext(defaultValue, options) {
  const id = Symbol("context");
  return {
    id,
    Provider: createProvider(id),
    defaultValue
  };
}
function useContext(context) {
  let value;
  return Owner && Owner.context && (value = Owner.context[context.id]) !== void 0 ? value : context.defaultValue;
}
function children(fn) {
  const children2 = createMemo(fn);
  const memo2 = createMemo(() => resolveChildren(children2()));
  memo2.toArray = () => {
    const c = memo2();
    return Array.isArray(c) ? c : c != null ? [c] : [];
  };
  return memo2;
}
let SuspenseContext;
function readSignal() {
  if (this.sources && this.state) {
    if (this.state === STALE) updateComputation(this);
    else {
      const updates = Updates;
      Updates = null;
      runUpdates(() => lookUpstream(this), false);
      Updates = updates;
    }
  }
  if (Listener) {
    const observers = this.observers;
    if (!observers || observers[observers.length - 1] !== Listener) {
      const sSlot = observers ? observers.length : 0;
      if (!Listener.sources) {
        Listener.sources = [this];
        Listener.sourceSlots = [sSlot];
      } else {
        Listener.sources.push(this);
        Listener.sourceSlots.push(sSlot);
      }
      if (!observers) {
        this.observers = [Listener];
        this.observerSlots = [Listener.sources.length - 1];
      } else {
        observers.push(Listener);
        this.observerSlots.push(Listener.sources.length - 1);
      }
    }
  }
  return this.value;
}
function writeSignal(node, value, isComp) {
  let current = node.value;
  if (!node.comparator || !node.comparator(current, value)) {
    node.value = value;
    if (node.observers && node.observers.length) {
      runUpdates(() => {
        for (let i = 0; i < node.observers.length; i += 1) {
          const o = node.observers[i];
          const TransitionRunning = Transition && Transition.running;
          if (TransitionRunning && Transition.disposed.has(o)) ;
          if (TransitionRunning ? !o.tState : !o.state) {
            if (o.pure) Updates.push(o);
            else Effects.push(o);
            if (o.observers) markDownstream(o);
          }
          if (!TransitionRunning) o.state = STALE;
        }
        if (Updates.length > 1e6) {
          Updates = [];
          if (IS_DEV) ;
          throw new Error();
        }
      }, false);
    }
  }
  return value;
}
function updateComputation(node) {
  if (!node.fn) return;
  cleanNode(node);
  const time = ExecCount;
  runComputation(node, node.value, time);
}
function runComputation(node, value, time) {
  let nextValue;
  const owner = Owner, listener = Listener;
  Listener = Owner = node;
  try {
    nextValue = node.fn(value);
  } catch (err) {
    if (node.pure) {
      {
        node.state = STALE;
        node.owned && node.owned.forEach(cleanNode);
        node.owned = null;
      }
    }
    node.updatedAt = time + 1;
    return handleError(err);
  } finally {
    Listener = listener;
    Owner = owner;
  }
  if (!node.updatedAt || node.updatedAt <= time) {
    if (node.updatedAt != null && "observers" in node) {
      writeSignal(node, nextValue);
    } else node.value = nextValue;
    node.updatedAt = time;
  }
}
function createComputation(fn, init, pure, state2 = STALE, options) {
  const c = {
    fn,
    state: state2,
    updatedAt: null,
    owned: null,
    sources: null,
    sourceSlots: null,
    cleanups: null,
    value: init,
    owner: Owner,
    context: Owner ? Owner.context : null,
    pure
  };
  if (Owner === null) ;
  else if (Owner !== UNOWNED) {
    {
      if (!Owner.owned) Owner.owned = [c];
      else Owner.owned.push(c);
    }
  }
  return c;
}
function runTop(node) {
  if (node.state === 0) return;
  if (node.state === PENDING) return lookUpstream(node);
  if (node.suspense && untrack(node.suspense.inFallback)) return node.suspense.effects.push(node);
  const ancestors = [node];
  while ((node = node.owner) && (!node.updatedAt || node.updatedAt < ExecCount)) {
    if (node.state) ancestors.push(node);
  }
  for (let i = ancestors.length - 1; i >= 0; i--) {
    node = ancestors[i];
    if (node.state === STALE) {
      updateComputation(node);
    } else if (node.state === PENDING) {
      const updates = Updates;
      Updates = null;
      runUpdates(() => lookUpstream(node, ancestors[0]), false);
      Updates = updates;
    }
  }
}
function runUpdates(fn, init) {
  if (Updates) return fn();
  let wait = false;
  if (!init) Updates = [];
  if (Effects) wait = true;
  else Effects = [];
  ExecCount++;
  try {
    const res = fn();
    completeUpdates(wait);
    return res;
  } catch (err) {
    if (!wait) Effects = null;
    Updates = null;
    handleError(err);
  }
}
function completeUpdates(wait) {
  if (Updates) {
    runQueue(Updates);
    Updates = null;
  }
  if (wait) return;
  const e = Effects;
  Effects = null;
  if (e.length) runUpdates(() => runEffects(e), false);
}
function runQueue(queue) {
  for (let i = 0; i < queue.length; i++) runTop(queue[i]);
}
function runUserEffects(queue) {
  let i, userLength = 0;
  for (i = 0; i < queue.length; i++) {
    const e = queue[i];
    if (!e.user) runTop(e);
    else queue[userLength++] = e;
  }
  for (i = 0; i < userLength; i++) runTop(queue[i]);
}
function lookUpstream(node, ignore) {
  node.state = 0;
  for (let i = 0; i < node.sources.length; i += 1) {
    const source = node.sources[i];
    if (source.sources) {
      const state2 = source.state;
      if (state2 === STALE) {
        if (source !== ignore && (!source.updatedAt || source.updatedAt < ExecCount)) runTop(source);
      } else if (state2 === PENDING) lookUpstream(source, ignore);
    }
  }
}
function markDownstream(node) {
  for (let i = 0; i < node.observers.length; i += 1) {
    const o = node.observers[i];
    if (!o.state) {
      o.state = PENDING;
      if (o.pure) Updates.push(o);
      else Effects.push(o);
      o.observers && markDownstream(o);
    }
  }
}
function cleanNode(node) {
  let i;
  if (node.sources) {
    while (node.sources.length) {
      const source = node.sources.pop(), index = node.sourceSlots.pop(), obs = source.observers;
      if (obs && obs.length) {
        const n = obs.pop(), s = source.observerSlots.pop();
        if (index < obs.length) {
          n.sourceSlots[s] = index;
          obs[index] = n;
          source.observerSlots[index] = s;
        }
      }
    }
  }
  if (node.tOwned) {
    for (i = node.tOwned.length - 1; i >= 0; i--) cleanNode(node.tOwned[i]);
    delete node.tOwned;
  }
  if (node.owned) {
    for (i = node.owned.length - 1; i >= 0; i--) cleanNode(node.owned[i]);
    node.owned = null;
  }
  if (node.cleanups) {
    for (i = node.cleanups.length - 1; i >= 0; i--) node.cleanups[i]();
    node.cleanups = null;
  }
  node.state = 0;
}
function castError(err) {
  if (err instanceof Error) return err;
  return new Error(typeof err === "string" ? err : "Unknown error", {
    cause: err
  });
}
function handleError(err, owner = Owner) {
  const error = castError(err);
  throw error;
}
function resolveChildren(children2) {
  if (typeof children2 === "function" && !children2.length) return resolveChildren(children2());
  if (Array.isArray(children2)) {
    const results = [];
    for (let i = 0; i < children2.length; i++) {
      const result = resolveChildren(children2[i]);
      if (Array.isArray(result)) {
        if (result.length < 32768) results.push.apply(results, result);
        else for (let j2 = 0; j2 < result.length; j2++) results.push(result[j2]);
      } else {
        results.push(result);
      }
    }
    return results;
  }
  return children2;
}
function createProvider(id, options) {
  return function provider(props) {
    let res;
    createRenderEffect(() => res = untrack(() => {
      Owner.context = {
        ...Owner.context,
        [id]: props.value
      };
      return children(() => props.children);
    }), void 0);
    return res;
  };
}
const FALLBACK = Symbol("fallback");
function dispose(d2) {
  for (let i = 0; i < d2.length; i++) d2[i]();
}
function mapArray(list2, mapFn, options = {}) {
  let items = [], mapped = [], disposers = [], len = 0, indexes = mapFn.length > 1 ? [] : null;
  onCleanup(() => dispose(disposers));
  return () => {
    let newItems = list2() || [], newLen = newItems.length, i, j2;
    newItems[$TRACK];
    return untrack(() => {
      let newIndices, newIndicesNext, temp, tempdisposers, tempIndexes, start, end, newEnd, item;
      if (newLen === 0) {
        if (len !== 0) {
          dispose(disposers);
          disposers = [];
          items = [];
          mapped = [];
          len = 0;
          indexes && (indexes = []);
        }
        if (options.fallback) {
          items = [FALLBACK];
          mapped[0] = createRoot((disposer) => {
            disposers[0] = disposer;
            return options.fallback();
          });
          len = 1;
        }
      } else if (len === 0) {
        mapped = new Array(newLen);
        for (j2 = 0; j2 < newLen; j2++) {
          items[j2] = newItems[j2];
          mapped[j2] = createRoot(mapper);
        }
        len = newLen;
      } else {
        temp = new Array(newLen);
        tempdisposers = new Array(newLen);
        indexes && (tempIndexes = new Array(newLen));
        for (start = 0, end = Math.min(len, newLen); start < end && items[start] === newItems[start]; start++) ;
        for (end = len - 1, newEnd = newLen - 1; end >= start && newEnd >= start && items[end] === newItems[newEnd]; end--, newEnd--) {
          temp[newEnd] = mapped[end];
          tempdisposers[newEnd] = disposers[end];
          indexes && (tempIndexes[newEnd] = indexes[end]);
        }
        newIndices = /* @__PURE__ */ new Map();
        newIndicesNext = new Array(newEnd + 1);
        for (j2 = newEnd; j2 >= start; j2--) {
          item = newItems[j2];
          i = newIndices.get(item);
          newIndicesNext[j2] = i === void 0 ? -1 : i;
          newIndices.set(item, j2);
        }
        for (i = start; i <= end; i++) {
          item = items[i];
          j2 = newIndices.get(item);
          if (j2 !== void 0 && j2 !== -1) {
            temp[j2] = mapped[i];
            tempdisposers[j2] = disposers[i];
            indexes && (tempIndexes[j2] = indexes[i]);
            j2 = newIndicesNext[j2];
            newIndices.set(item, j2);
          } else disposers[i]();
        }
        for (j2 = start; j2 < newLen; j2++) {
          if (j2 in temp) {
            mapped[j2] = temp[j2];
            disposers[j2] = tempdisposers[j2];
            if (indexes) {
              indexes[j2] = tempIndexes[j2];
              indexes[j2](j2);
            }
          } else mapped[j2] = createRoot(mapper);
        }
        mapped = mapped.slice(0, len = newLen);
        items = newItems.slice(0);
      }
      return mapped;
    });
    function mapper(disposer) {
      disposers[j2] = disposer;
      if (indexes) {
        const [s, set] = createSignal(j2);
        indexes[j2] = set;
        return mapFn(newItems[j2], s);
      }
      return mapFn(newItems[j2]);
    }
  };
}
function createComponent(Comp, props) {
  return untrack(() => Comp(props || {}));
}
function trueFn() {
  return true;
}
const propTraps = {
  get(_2, property, receiver) {
    if (property === $PROXY) return receiver;
    return _2.get(property);
  },
  has(_2, property) {
    if (property === $PROXY) return true;
    return _2.has(property);
  },
  set: trueFn,
  deleteProperty: trueFn,
  getOwnPropertyDescriptor(_2, property) {
    return {
      configurable: true,
      enumerable: true,
      get() {
        return _2.get(property);
      },
      set: trueFn,
      deleteProperty: trueFn
    };
  },
  ownKeys(_2) {
    return _2.keys();
  }
};
function resolveSource(s) {
  return !(s = typeof s === "function" ? s() : s) ? {} : s;
}
function resolveSources() {
  for (let i = 0, length = this.length; i < length; ++i) {
    const v2 = this[i]();
    if (v2 !== void 0) return v2;
  }
}
function mergeProps(...sources) {
  let proxy = false;
  for (let i = 0; i < sources.length; i++) {
    const s = sources[i];
    proxy = proxy || !!s && $PROXY in s;
    sources[i] = typeof s === "function" ? (proxy = true, createMemo(s)) : s;
  }
  if (SUPPORTS_PROXY && proxy) {
    return new Proxy({
      get(property) {
        for (let i = sources.length - 1; i >= 0; i--) {
          const v2 = resolveSource(sources[i])[property];
          if (v2 !== void 0) return v2;
        }
      },
      has(property) {
        for (let i = sources.length - 1; i >= 0; i--) {
          if (property in resolveSource(sources[i])) return true;
        }
        return false;
      },
      keys() {
        const keys = [];
        for (let i = 0; i < sources.length; i++) keys.push(...Object.keys(resolveSource(sources[i])));
        return [...new Set(keys)];
      }
    }, propTraps);
  }
  const sourcesMap = {};
  const defined = /* @__PURE__ */ Object.create(null);
  for (let i = sources.length - 1; i >= 0; i--) {
    const source = sources[i];
    if (!source) continue;
    const sourceKeys = Object.getOwnPropertyNames(source);
    for (let i2 = sourceKeys.length - 1; i2 >= 0; i2--) {
      const key = sourceKeys[i2];
      if (key === "__proto__" || key === "constructor") continue;
      const desc = Object.getOwnPropertyDescriptor(source, key);
      if (!defined[key]) {
        defined[key] = desc.get ? {
          enumerable: true,
          configurable: true,
          get: resolveSources.bind(sourcesMap[key] = [desc.get.bind(source)])
        } : desc.value !== void 0 ? desc : void 0;
      } else {
        const sources2 = sourcesMap[key];
        if (sources2) {
          if (desc.get) sources2.push(desc.get.bind(source));
          else if (desc.value !== void 0) sources2.push(() => desc.value);
        }
      }
    }
  }
  const target = {};
  const definedKeys = Object.keys(defined);
  for (let i = definedKeys.length - 1; i >= 0; i--) {
    const key = definedKeys[i], desc = defined[key];
    if (desc && desc.get) Object.defineProperty(target, key, desc);
    else target[key] = desc ? desc.value : void 0;
  }
  return target;
}
function splitProps(props, ...keys) {
  const len = keys.length;
  if (SUPPORTS_PROXY && $PROXY in props) {
    const blocked = len > 1 ? keys.flat() : keys[0];
    const res = keys.map((k) => {
      return new Proxy({
        get(property) {
          return k.includes(property) ? props[property] : void 0;
        },
        has(property) {
          return k.includes(property) && property in props;
        },
        keys() {
          return k.filter((property) => property in props);
        }
      }, propTraps);
    });
    res.push(new Proxy({
      get(property) {
        return blocked.includes(property) ? void 0 : props[property];
      },
      has(property) {
        return blocked.includes(property) ? false : property in props;
      },
      keys() {
        return Object.keys(props).filter((k) => !blocked.includes(k));
      }
    }, propTraps));
    return res;
  }
  const objects = [];
  for (let i = 0; i <= len; i++) {
    objects[i] = {};
  }
  for (const propName of Object.getOwnPropertyNames(props)) {
    let keyIndex = len;
    for (let i = 0; i < keys.length; i++) {
      if (keys[i].includes(propName)) {
        keyIndex = i;
        break;
      }
    }
    const desc = Object.getOwnPropertyDescriptor(props, propName);
    const isDefaultDesc = !desc.get && !desc.set && desc.enumerable && desc.writable && desc.configurable;
    isDefaultDesc ? objects[keyIndex][propName] = desc.value : Object.defineProperty(objects[keyIndex], propName, desc);
  }
  return objects;
}
const narrowedError = (name) => `Stale read from <${name}>.`;
function For(props) {
  const fallback = "fallback" in props && {
    fallback: () => props.fallback
  };
  return createMemo(mapArray(() => props.each, props.children, fallback || void 0));
}
function Show(props) {
  const keyed = props.keyed;
  const conditionValue = createMemo(() => props.when, void 0, void 0);
  const condition = keyed ? conditionValue : createMemo(conditionValue, void 0, {
    equals: (a, b2) => !a === !b2
  });
  return createMemo(() => {
    const c = condition();
    if (c) {
      const child = props.children;
      const fn = typeof child === "function" && child.length > 0;
      return fn ? untrack(() => child(keyed ? c : () => {
        if (!untrack(condition)) throw narrowedError("Show");
        return conditionValue();
      })) : child;
    }
    return props.fallback;
  }, void 0, void 0);
}
function Switch(props) {
  const chs = children(() => props.children);
  const switchFunc = createMemo(() => {
    const ch = chs();
    const mps = Array.isArray(ch) ? ch : [ch];
    let func = () => void 0;
    for (let i = 0; i < mps.length; i++) {
      const index = i;
      const mp = mps[i];
      const prevFunc = func;
      const conditionValue = createMemo(() => prevFunc() ? void 0 : mp.when, void 0, void 0);
      const condition = mp.keyed ? conditionValue : createMemo(conditionValue, void 0, {
        equals: (a, b2) => !a === !b2
      });
      func = () => prevFunc() || (condition() ? [index, conditionValue, mp] : void 0);
    }
    return func;
  });
  return createMemo(() => {
    const sel = switchFunc()();
    if (!sel) return props.fallback;
    const [index, conditionValue, mp] = sel;
    const child = mp.children;
    const fn = typeof child === "function" && child.length > 0;
    return fn ? untrack(() => child(mp.keyed ? conditionValue() : () => {
      if (untrack(switchFunc)()?.[0] !== index) throw narrowedError("Match");
      return conditionValue();
    })) : child;
  }, void 0, void 0);
}
function Match(props) {
  return props;
}
const booleans = [
  "allowfullscreen",
  "async",
  "alpha",
  "autofocus",
  "autoplay",
  "checked",
  "controls",
  "default",
  "disabled",
  "formnovalidate",
  "hidden",
  "indeterminate",
  "inert",
  "ismap",
  "loop",
  "multiple",
  "muted",
  "nomodule",
  "novalidate",
  "open",
  "playsinline",
  "readonly",
  "required",
  "reversed",
  "seamless",
  "selected",
  "adauctionheaders",
  "browsingtopics",
  "credentialless",
  "defaultchecked",
  "defaultmuted",
  "defaultselected",
  "defer",
  "disablepictureinpicture",
  "disableremoteplayback",
  "preservespitch",
  "shadowrootclonable",
  "shadowrootcustomelementregistry",
  "shadowrootdelegatesfocus",
  "shadowrootserializable",
  "sharedstoragewritable"
];
const Properties = /* @__PURE__ */ new Set([
  "className",
  "value",
  "readOnly",
  "noValidate",
  "formNoValidate",
  "isMap",
  "noModule",
  "playsInline",
  "adAuctionHeaders",
  "allowFullscreen",
  "browsingTopics",
  "defaultChecked",
  "defaultMuted",
  "defaultSelected",
  "disablePictureInPicture",
  "disableRemotePlayback",
  "preservesPitch",
  "shadowRootClonable",
  "shadowRootCustomElementRegistry",
  "shadowRootDelegatesFocus",
  "shadowRootSerializable",
  "sharedStorageWritable",
  ...booleans
]);
const ChildProperties = /* @__PURE__ */ new Set(["innerHTML", "textContent", "innerText", "children"]);
const Aliases = /* @__PURE__ */ Object.assign(/* @__PURE__ */ Object.create(null), {
  className: "class",
  htmlFor: "for"
});
const PropAliases = /* @__PURE__ */ Object.assign(/* @__PURE__ */ Object.create(null), {
  class: "className",
  novalidate: {
    $: "noValidate",
    FORM: 1
  },
  formnovalidate: {
    $: "formNoValidate",
    BUTTON: 1,
    INPUT: 1
  },
  ismap: {
    $: "isMap",
    IMG: 1
  },
  nomodule: {
    $: "noModule",
    SCRIPT: 1
  },
  playsinline: {
    $: "playsInline",
    VIDEO: 1
  },
  readonly: {
    $: "readOnly",
    INPUT: 1,
    TEXTAREA: 1
  },
  adauctionheaders: {
    $: "adAuctionHeaders",
    IFRAME: 1
  },
  allowfullscreen: {
    $: "allowFullscreen",
    IFRAME: 1
  },
  browsingtopics: {
    $: "browsingTopics",
    IMG: 1
  },
  defaultchecked: {
    $: "defaultChecked",
    INPUT: 1
  },
  defaultmuted: {
    $: "defaultMuted",
    AUDIO: 1,
    VIDEO: 1
  },
  defaultselected: {
    $: "defaultSelected",
    OPTION: 1
  },
  disablepictureinpicture: {
    $: "disablePictureInPicture",
    VIDEO: 1
  },
  disableremoteplayback: {
    $: "disableRemotePlayback",
    AUDIO: 1,
    VIDEO: 1
  },
  preservespitch: {
    $: "preservesPitch",
    AUDIO: 1,
    VIDEO: 1
  },
  shadowrootclonable: {
    $: "shadowRootClonable",
    TEMPLATE: 1
  },
  shadowrootdelegatesfocus: {
    $: "shadowRootDelegatesFocus",
    TEMPLATE: 1
  },
  shadowrootserializable: {
    $: "shadowRootSerializable",
    TEMPLATE: 1
  },
  sharedstoragewritable: {
    $: "sharedStorageWritable",
    IFRAME: 1,
    IMG: 1
  }
});
function getPropAlias(prop, tagName) {
  const a = PropAliases[prop];
  return typeof a === "object" ? a[tagName] ? a["$"] : void 0 : a;
}
const DelegatedEvents = /* @__PURE__ */ new Set(["beforeinput", "click", "dblclick", "contextmenu", "focusin", "focusout", "input", "keydown", "keyup", "mousedown", "mousemove", "mouseout", "mouseover", "mouseup", "pointerdown", "pointermove", "pointerout", "pointerover", "pointerup", "touchend", "touchmove", "touchstart"]);
const memo = (fn) => createMemo(() => fn());
function reconcileArrays(parentNode, a, b2) {
  let bLength = b2.length, aEnd = a.length, bEnd = bLength, aStart = 0, bStart = 0, after = a[aEnd - 1].nextSibling, map = null;
  while (aStart < aEnd || bStart < bEnd) {
    if (a[aStart] === b2[bStart]) {
      aStart++;
      bStart++;
      continue;
    }
    while (a[aEnd - 1] === b2[bEnd - 1]) {
      aEnd--;
      bEnd--;
    }
    if (aEnd === aStart) {
      const node = bEnd < bLength ? bStart ? b2[bStart - 1].nextSibling : b2[bEnd - bStart] : after;
      while (bStart < bEnd) parentNode.insertBefore(b2[bStart++], node);
    } else if (bEnd === bStart) {
      while (aStart < aEnd) {
        if (!map || !map.has(a[aStart])) a[aStart].remove();
        aStart++;
      }
    } else if (a[aStart] === b2[bEnd - 1] && b2[bStart] === a[aEnd - 1]) {
      const node = a[--aEnd].nextSibling;
      parentNode.insertBefore(b2[bStart++], a[aStart++].nextSibling);
      parentNode.insertBefore(b2[--bEnd], node);
      a[aEnd] = b2[bEnd];
    } else {
      if (!map) {
        map = /* @__PURE__ */ new Map();
        let i = bStart;
        while (i < bEnd) map.set(b2[i], i++);
      }
      const index = map.get(a[aStart]);
      if (index != null) {
        if (bStart < index && index < bEnd) {
          let i = aStart, sequence = 1, t;
          while (++i < aEnd && i < bEnd) {
            if ((t = map.get(a[i])) == null || t !== index + sequence) break;
            sequence++;
          }
          if (sequence > index - bStart) {
            const node = a[aStart];
            while (bStart < index) parentNode.insertBefore(b2[bStart++], node);
          } else parentNode.replaceChild(b2[bStart++], a[aStart++]);
        } else aStart++;
      } else a[aStart++].remove();
    }
  }
}
const $$EVENTS = "_$DX_DELEGATE";
function render(code, element, init, options = {}) {
  let disposer;
  createRoot((dispose2) => {
    disposer = dispose2;
    element === document ? code() : insert(element, code(), element.firstChild ? null : void 0, init);
  }, options.owner);
  return () => {
    disposer();
    element.textContent = "";
  };
}
function template(html2, isImportNode, isSVG, isMathML) {
  let node;
  const create2 = () => {
    const t = document.createElement("template");
    t.innerHTML = html2;
    return t.content.firstChild;
  };
  const fn = () => (node || (node = create2())).cloneNode(true);
  fn.cloneNode = fn;
  return fn;
}
function delegateEvents(eventNames, document2 = window.document) {
  const e = document2[$$EVENTS] || (document2[$$EVENTS] = /* @__PURE__ */ new Set());
  for (let i = 0, l3 = eventNames.length; i < l3; i++) {
    const name = eventNames[i];
    if (!e.has(name)) {
      e.add(name);
      document2.addEventListener(name, eventHandler);
    }
  }
}
function setAttribute(node, name, value) {
  if (value == null) node.removeAttribute(name);
  else node.setAttribute(name, value);
}
function setBoolAttribute(node, name, value) {
  value ? node.setAttribute(name, "") : node.removeAttribute(name);
}
function className(node, value) {
  if (value == null) node.removeAttribute("class");
  else node.className = value;
}
function addEventListener(node, name, handler, delegate) {
  if (delegate) {
    if (Array.isArray(handler)) {
      node[`$$${name}`] = handler[0];
      node[`$$${name}Data`] = handler[1];
    } else node[`$$${name}`] = handler;
  } else if (Array.isArray(handler)) {
    const handlerFn = handler[0];
    node.addEventListener(name, handler[0] = (e) => handlerFn.call(node, handler[1], e));
  } else node.addEventListener(name, handler, typeof handler !== "function" && handler);
}
function classList(node, value, prev = {}) {
  const classKeys = Object.keys(value || {}), prevKeys = Object.keys(prev);
  let i, len;
  for (i = 0, len = prevKeys.length; i < len; i++) {
    const key = prevKeys[i];
    if (!key || key === "undefined" || value[key]) continue;
    toggleClassKey(node, key, false);
    delete prev[key];
  }
  for (i = 0, len = classKeys.length; i < len; i++) {
    const key = classKeys[i], classValue = !!value[key];
    if (!key || key === "undefined" || prev[key] === classValue || !classValue) continue;
    toggleClassKey(node, key, true);
    prev[key] = classValue;
  }
  return prev;
}
function style(node, value, prev) {
  if (!value) return prev ? setAttribute(node, "style") : value;
  const nodeStyle = node.style;
  if (typeof value === "string") return nodeStyle.cssText = value;
  typeof prev === "string" && (nodeStyle.cssText = prev = void 0);
  prev || (prev = {});
  value || (value = {});
  let v2, s;
  for (s in prev) {
    value[s] == null && nodeStyle.removeProperty(s);
    delete prev[s];
  }
  for (s in value) {
    v2 = value[s];
    if (v2 !== prev[s]) {
      nodeStyle.setProperty(s, v2);
      prev[s] = v2;
    }
  }
  return prev;
}
function setStyleProperty(node, name, value) {
  value != null ? node.style.setProperty(name, value) : node.style.removeProperty(name);
}
function spread(node, props = {}, isSVG, skipChildren) {
  const prevProps = {};
  {
    createRenderEffect(() => prevProps.children = insertExpression(node, props.children, prevProps.children));
  }
  createRenderEffect(() => typeof props.ref === "function" && use(props.ref, node));
  createRenderEffect(() => assign(node, props, isSVG, true, prevProps, true));
  return prevProps;
}
function use(fn, element, arg) {
  return untrack(() => fn(element, arg));
}
function insert(parent, accessor, marker, initial) {
  if (marker !== void 0 && !initial) initial = [];
  if (typeof accessor !== "function") return insertExpression(parent, accessor, initial, marker);
  createRenderEffect((current) => insertExpression(parent, accessor(), current, marker), initial);
}
function assign(node, props, isSVG, skipChildren, prevProps = {}, skipRef = false) {
  props || (props = {});
  for (const prop in prevProps) {
    if (!(prop in props)) {
      if (prop === "children") continue;
      prevProps[prop] = assignProp(node, prop, null, prevProps[prop], isSVG, skipRef, props);
    }
  }
  for (const prop in props) {
    if (prop === "children") {
      continue;
    }
    const value = props[prop];
    prevProps[prop] = assignProp(node, prop, value, prevProps[prop], isSVG, skipRef, props);
  }
}
function toPropertyName(name) {
  return name.toLowerCase().replace(/-([a-z])/g, (_2, w2) => w2.toUpperCase());
}
function toggleClassKey(node, key, value) {
  const classNames = key.trim().split(/\s+/);
  for (let i = 0, nameLen = classNames.length; i < nameLen; i++) node.classList.toggle(classNames[i], value);
}
function assignProp(node, prop, value, prev, isSVG, skipRef, props) {
  let isCE, isProp, isChildProp, propAlias, forceProp;
  if (prop === "style") return style(node, value, prev);
  if (prop === "classList") return classList(node, value, prev);
  if (value === prev) return prev;
  if (prop === "ref") {
    if (!skipRef) value(node);
  } else if (prop.slice(0, 3) === "on:") {
    const e = prop.slice(3);
    prev && node.removeEventListener(e, prev, typeof prev !== "function" && prev);
    value && node.addEventListener(e, value, typeof value !== "function" && value);
  } else if (prop.slice(0, 10) === "oncapture:") {
    const e = prop.slice(10);
    prev && node.removeEventListener(e, prev, true);
    value && node.addEventListener(e, value, true);
  } else if (prop.slice(0, 2) === "on") {
    const name = prop.slice(2).toLowerCase();
    const delegate = DelegatedEvents.has(name);
    if (!delegate && prev) {
      const h = Array.isArray(prev) ? prev[0] : prev;
      node.removeEventListener(name, h);
    }
    if (delegate || value) {
      addEventListener(node, name, value, delegate);
      delegate && delegateEvents([name]);
    }
  } else if (prop.slice(0, 5) === "attr:") {
    setAttribute(node, prop.slice(5), value);
  } else if (prop.slice(0, 5) === "bool:") {
    setBoolAttribute(node, prop.slice(5), value);
  } else if ((forceProp = prop.slice(0, 5) === "prop:") || (isChildProp = ChildProperties.has(prop)) || ((propAlias = getPropAlias(prop, node.tagName)) || (isProp = Properties.has(prop))) || (isCE = node.nodeName.includes("-") || "is" in props)) {
    if (forceProp) {
      prop = prop.slice(5);
      isProp = true;
    }
    if (prop === "class" || prop === "className") className(node, value);
    else if (isCE && !isProp && !isChildProp) node[toPropertyName(prop)] = value;
    else node[propAlias || prop] = value;
  } else {
    setAttribute(node, Aliases[prop] || prop, value);
  }
  return value;
}
function eventHandler(e) {
  let node = e.target;
  const key = `$$${e.type}`;
  const oriTarget = e.target;
  const oriCurrentTarget = e.currentTarget;
  const retarget = (value) => Object.defineProperty(e, "target", {
    configurable: true,
    value
  });
  const handleNode = () => {
    const handler = node[key];
    if (handler && !node.disabled) {
      const data = node[`${key}Data`];
      data !== void 0 ? handler.call(node, data, e) : handler.call(node, e);
      if (e.cancelBubble) return;
    }
    node.host && typeof node.host !== "string" && !node.host._$host && node.contains(e.target) && retarget(node.host);
    return true;
  };
  const walkUpTree = () => {
    while (handleNode() && (node = node._$host || node.parentNode || node.host)) ;
  };
  Object.defineProperty(e, "currentTarget", {
    configurable: true,
    get() {
      return node || document;
    }
  });
  if (e.composedPath) {
    const path = e.composedPath();
    retarget(path[0]);
    for (let i = 0; i < path.length - 2; i++) {
      node = path[i];
      if (!handleNode()) break;
      if (node._$host) {
        node = node._$host;
        walkUpTree();
        break;
      }
      if (node.parentNode === oriCurrentTarget) {
        break;
      }
    }
  } else walkUpTree();
  retarget(oriTarget);
}
function insertExpression(parent, value, current, marker, unwrapArray) {
  while (typeof current === "function") current = current();
  if (value === current) return current;
  const t = typeof value, multi = marker !== void 0;
  parent = multi && current[0] && current[0].parentNode || parent;
  if (t === "string" || t === "number") {
    if (t === "number") {
      value = value.toString();
      if (value === current) return current;
    }
    if (multi) {
      let node = current[0];
      if (node && node.nodeType === 3) {
        node.data !== value && (node.data = value);
      } else node = document.createTextNode(value);
      current = cleanChildren(parent, current, marker, node);
    } else {
      if (current !== "" && typeof current === "string") {
        current = parent.firstChild.data = value;
      } else current = parent.textContent = value;
    }
  } else if (value == null || t === "boolean") {
    current = cleanChildren(parent, current, marker);
  } else if (t === "function") {
    createRenderEffect(() => {
      let v2 = value();
      while (typeof v2 === "function") v2 = v2();
      current = insertExpression(parent, v2, current, marker);
    });
    return () => current;
  } else if (Array.isArray(value)) {
    const array = [];
    const currentArray = current && Array.isArray(current);
    if (normalizeIncomingArray(array, value, current, unwrapArray)) {
      createRenderEffect(() => current = insertExpression(parent, array, current, marker, true));
      return () => current;
    }
    if (array.length === 0) {
      current = cleanChildren(parent, current, marker);
      if (multi) return current;
    } else if (currentArray) {
      if (current.length === 0) {
        appendNodes(parent, array, marker);
      } else reconcileArrays(parent, current, array);
    } else {
      current && cleanChildren(parent);
      appendNodes(parent, array);
    }
    current = array;
  } else if (value.nodeType) {
    if (Array.isArray(current)) {
      if (multi) return current = cleanChildren(parent, current, marker, value);
      cleanChildren(parent, current, null, value);
    } else if (current == null || current === "" || !parent.firstChild) {
      parent.appendChild(value);
    } else parent.replaceChild(value, parent.firstChild);
    current = value;
  } else ;
  return current;
}
function normalizeIncomingArray(normalized, array, current, unwrap) {
  let dynamic = false;
  for (let i = 0, len = array.length; i < len; i++) {
    let item = array[i], prev = current && current[normalized.length], t;
    if (item == null || item === true || item === false) ;
    else if ((t = typeof item) === "object" && item.nodeType) {
      normalized.push(item);
    } else if (Array.isArray(item)) {
      dynamic = normalizeIncomingArray(normalized, item, prev) || dynamic;
    } else if (t === "function") {
      if (unwrap) {
        while (typeof item === "function") item = item();
        dynamic = normalizeIncomingArray(normalized, Array.isArray(item) ? item : [item], Array.isArray(prev) ? prev : [prev]) || dynamic;
      } else {
        normalized.push(item);
        dynamic = true;
      }
    } else {
      const value = String(item);
      if (prev && prev.nodeType === 3 && prev.data === value) normalized.push(prev);
      else normalized.push(document.createTextNode(value));
    }
  }
  return dynamic;
}
function appendNodes(parent, array, marker = null) {
  for (let i = 0, len = array.length; i < len; i++) parent.insertBefore(array[i], marker);
}
function cleanChildren(parent, current, marker, replacement) {
  if (marker === void 0) return parent.textContent = "";
  const node = replacement || document.createTextNode("");
  if (current.length) {
    let inserted = false;
    for (let i = current.length - 1; i >= 0; i--) {
      const el = current[i];
      if (node !== el) {
        const isParent = el.parentNode === parent;
        if (!inserted && !i) isParent ? parent.replaceChild(node, el) : parent.insertBefore(node, marker);
        else isParent && el.remove();
      } else inserted = true;
    }
  } else parent.insertBefore(node, marker);
  return [node];
}
const voidFn = () => void 0;
const isServer = false;
function createBeforeLeave() {
  let listeners = /* @__PURE__ */ new Set();
  function subscribe(listener) {
    listeners.add(listener);
    return () => listeners.delete(listener);
  }
  let ignore = false;
  function confirm2(to, options) {
    if (ignore)
      return !(ignore = false);
    const e = {
      to,
      options,
      defaultPrevented: false,
      preventDefault: () => e.defaultPrevented = true
    };
    for (const l3 of listeners)
      l3.listener({
        ...e,
        from: l3.location,
        retry: (force) => {
          force && (ignore = true);
          l3.navigate(to, { ...options, resolve: false });
        }
      });
    return !e.defaultPrevented;
  }
  return {
    subscribe,
    confirm: confirm2
  };
}
let depth;
function saveCurrentDepth() {
  if (!window.history.state || window.history.state._depth == null) {
    window.history.replaceState({ ...window.history.state, _depth: window.history.length - 1 }, "");
  }
  depth = window.history.state._depth;
}
{
  saveCurrentDepth();
}
function keepDepth(state2) {
  return {
    ...state2,
    _depth: window.history.state && window.history.state._depth
  };
}
function notifyIfNotBlocked(notify, block) {
  let ignore = false;
  return () => {
    const prevDepth = depth;
    saveCurrentDepth();
    const delta = prevDepth == null ? null : depth - prevDepth;
    if (ignore) {
      ignore = false;
      return;
    }
    if (delta && block(delta)) {
      ignore = true;
      window.history.go(-delta);
    } else {
      notify();
    }
  };
}
const hasSchemeRegex = /^(?:[a-z0-9]+:)?\/\//i;
const trimPathRegex = /^\/+|(\/)\/+$/g;
const mockBase = "http://sr";
function normalizePath(path, omitSlash = false) {
  const s = path.replace(trimPathRegex, "$1");
  return s ? omitSlash || /^[?#]/.test(s) ? s : "/" + s : "";
}
function resolvePath(base, path, from) {
  if (hasSchemeRegex.test(path)) {
    return void 0;
  }
  const basePath = normalizePath(base);
  const fromPath = from && normalizePath(from);
  let result = "";
  if (!fromPath || path.startsWith("/")) {
    result = basePath;
  } else if (fromPath.toLowerCase().indexOf(basePath.toLowerCase()) !== 0) {
    result = basePath + fromPath;
  } else {
    result = fromPath;
  }
  return (result || "/") + normalizePath(path, !result);
}
function invariant(value, message) {
  if (value == null) {
    throw new Error(message);
  }
  return value;
}
function joinPaths(from, to) {
  return normalizePath(from).replace(/\/*(\*.*)?$/g, "") + normalizePath(to);
}
function extractSearchParams(url) {
  const params = {};
  url.searchParams.forEach((value, key) => {
    if (key in params) {
      if (Array.isArray(params[key]))
        params[key].push(value);
      else
        params[key] = [params[key], value];
    } else
      params[key] = value;
  });
  return params;
}
function createMatcher(path, partial, matchFilters) {
  const [pattern, splat] = path.split("/*", 2);
  const segments = pattern.split("/").filter(Boolean);
  const len = segments.length;
  return (location) => {
    const locSegments = location.split("/").filter(Boolean);
    const lenDiff = locSegments.length - len;
    if (lenDiff < 0 || lenDiff > 0 && splat === void 0 && !partial) {
      return null;
    }
    const match = {
      path: len ? "" : "/",
      params: {}
    };
    const matchFilter = (s) => matchFilters === void 0 ? void 0 : matchFilters[s];
    for (let i = 0; i < len; i++) {
      const segment = segments[i];
      const dynamic = segment[0] === ":";
      const locSegment = dynamic ? locSegments[i] : locSegments[i].toLowerCase();
      const key = dynamic ? segment.slice(1) : segment.toLowerCase();
      if (dynamic && matchSegment(locSegment, matchFilter(key))) {
        match.params[key] = locSegment;
      } else if (dynamic || !matchSegment(locSegment, key)) {
        return null;
      }
      match.path += `/${locSegment}`;
    }
    if (splat) {
      const remainder = lenDiff ? locSegments.slice(-lenDiff).join("/") : "";
      if (matchSegment(remainder, matchFilter(splat))) {
        match.params[splat] = remainder;
      } else {
        return null;
      }
    }
    return match;
  };
}
function matchSegment(input, filter) {
  const isEqual = (s) => s === input;
  if (filter === void 0) {
    return true;
  } else if (typeof filter === "string") {
    return isEqual(filter);
  } else if (typeof filter === "function") {
    return filter(input);
  } else if (Array.isArray(filter)) {
    return filter.some(isEqual);
  } else if (filter instanceof RegExp) {
    return filter.test(input);
  }
  return false;
}
function scoreRoute(route) {
  const [pattern, splat] = route.pattern.split("/*", 2);
  const segments = pattern.split("/").filter(Boolean);
  return segments.reduce((score, segment) => score + (segment.startsWith(":") ? 2 : 3), segments.length - (splat === void 0 ? 0 : 1));
}
function createMemoObject(fn) {
  const map = /* @__PURE__ */ new Map();
  const owner = getOwner();
  return new Proxy({}, {
    get(_2, property) {
      if (!map.has(property)) {
        runWithOwner(owner, () => map.set(property, createMemo(() => fn()[property])));
      }
      return map.get(property)();
    },
    getOwnPropertyDescriptor() {
      return {
        enumerable: true,
        configurable: true
      };
    },
    ownKeys() {
      return Reflect.ownKeys(fn());
    }
  });
}
function expandOptionals(pattern) {
  let match = /(\/?\:[^\/]+)\?/.exec(pattern);
  if (!match)
    return [pattern];
  let prefix = pattern.slice(0, match.index);
  let suffix = pattern.slice(match.index + match[0].length);
  const prefixes = [prefix, prefix += match[1]];
  while (match = /^(\/\:[^\/]+)\?/.exec(suffix)) {
    prefixes.push(prefix += match[1]);
    suffix = suffix.slice(match[0].length);
  }
  return expandOptionals(suffix).reduce((results, expansion) => [...results, ...prefixes.map((p) => p + expansion)], []);
}
const MAX_REDIRECTS = 100;
const RouterContextObj = createContext();
const RouteContextObj = createContext();
const useRouter = () => invariant(useContext(RouterContextObj), "<A> and 'use' router primitives can be only used inside a Route.");
const useRoute = () => useContext(RouteContextObj) || useRouter().base;
const useResolvedPath = (path) => {
  const route = useRoute();
  return createMemo(() => route.resolvePath(path()));
};
const useHref = (to) => {
  const router = useRouter();
  return createMemo(() => {
    const to_ = to();
    return to_ !== void 0 ? router.renderPath(to_) : to_;
  });
};
const useNavigate = () => useRouter().navigatorFactory();
const useLocation = () => useRouter().location;
const useParams = () => useRouter().params;
function createRoutes(routeDef, base = "") {
  const { component, preload: preload2, load, children: children2, info } = routeDef;
  const isLeaf = !children2 || Array.isArray(children2) && !children2.length;
  const shared = {
    key: routeDef,
    component,
    preload: preload2 || load,
    info
  };
  return asArray(routeDef.path).reduce((acc, originalPath) => {
    for (const expandedPath of expandOptionals(originalPath)) {
      const path = joinPaths(base, expandedPath);
      let pattern = isLeaf ? path : path.split("/*", 1)[0];
      pattern = pattern.split("/").map((s) => {
        return s.startsWith(":") || s.startsWith("*") ? s : encodeURIComponent(s);
      }).join("/");
      acc.push({
        ...shared,
        originalPath,
        pattern,
        matcher: createMatcher(pattern, !isLeaf, routeDef.matchFilters)
      });
    }
    return acc;
  }, []);
}
function createBranch(routes, index = 0) {
  return {
    routes,
    score: scoreRoute(routes[routes.length - 1]) * 1e4 - index,
    matcher(location) {
      const matches = [];
      for (let i = routes.length - 1; i >= 0; i--) {
        const route = routes[i];
        const match = route.matcher(location);
        if (!match) {
          return null;
        }
        matches.unshift({
          ...match,
          route
        });
      }
      return matches;
    }
  };
}
function asArray(value) {
  return Array.isArray(value) ? value : [value];
}
function createBranches(routeDef, base = "", stack = [], branches = []) {
  const routeDefs = asArray(routeDef);
  for (let i = 0, len = routeDefs.length; i < len; i++) {
    const def = routeDefs[i];
    if (def && typeof def === "object") {
      if (!def.hasOwnProperty("path"))
        def.path = "";
      const routes = createRoutes(def, base);
      for (const route of routes) {
        stack.push(route);
        const isEmptyArray = Array.isArray(def.children) && def.children.length === 0;
        if (def.children && !isEmptyArray) {
          createBranches(def.children, route.pattern, stack, branches);
        } else {
          const branch = createBranch([...stack], branches.length);
          branches.push(branch);
        }
        stack.pop();
      }
    }
  }
  return stack.length ? branches : branches.sort((a, b2) => b2.score - a.score);
}
function getRouteMatches(branches, location) {
  for (let i = 0, len = branches.length; i < len; i++) {
    const match = branches[i].matcher(location);
    if (match) {
      return match;
    }
  }
  return [];
}
function createLocation(path, state2, queryWrapper) {
  const origin = new URL(mockBase);
  const url = createMemo((prev) => {
    const path_ = path();
    try {
      return new URL(path_, origin);
    } catch (err) {
      console.error(`Invalid path ${path_}`);
      return prev;
    }
  }, origin, {
    equals: (a, b2) => a.href === b2.href
  });
  const pathname = createMemo(() => url().pathname);
  const search = createMemo(() => url().search, true);
  const hash = createMemo(() => url().hash);
  const key = () => "";
  const queryFn = on(search, () => extractSearchParams(url()));
  return {
    get pathname() {
      return pathname();
    },
    get search() {
      return search();
    },
    get hash() {
      return hash();
    },
    get state() {
      return state2();
    },
    get key() {
      return key();
    },
    query: queryWrapper ? queryWrapper(queryFn) : createMemoObject(queryFn)
  };
}
let intent;
function getIntent() {
  return intent;
}
function setInPreloadFn(value) {
}
function createRouterContext(integration, branches, getContext, options = {}) {
  const { signal: [source, setSource], utils = {} } = integration;
  const parsePath = utils.parsePath || ((p) => p);
  const renderPath = utils.renderPath || ((p) => p);
  const beforeLeave = utils.beforeLeave || createBeforeLeave();
  const basePath = resolvePath("", options.base || "");
  if (basePath === void 0) {
    throw new Error(`${basePath} is not a valid base path`);
  } else if (basePath && !source().value) {
    setSource({ value: basePath, replace: true, scroll: false });
  }
  const [isRouting, setIsRouting] = createSignal(false);
  let lastTransitionTarget;
  const transition = (newIntent, newTarget) => {
    if (newTarget.value === reference() && newTarget.state === state2())
      return;
    if (lastTransitionTarget === void 0)
      setIsRouting(true);
    intent = newIntent;
    lastTransitionTarget = newTarget;
    startTransition(() => {
      if (lastTransitionTarget !== newTarget)
        return;
      setReference(lastTransitionTarget.value);
      setState(lastTransitionTarget.state);
      submissions[1]([]);
    }).finally(() => {
      if (lastTransitionTarget !== newTarget)
        return;
      batch(() => {
        intent = void 0;
        if (newIntent === "navigate")
          navigateEnd(lastTransitionTarget);
        setIsRouting(false);
        lastTransitionTarget = void 0;
      });
    });
  };
  const [reference, setReference] = createSignal(source().value);
  const [state2, setState] = createSignal(source().state);
  const location = createLocation(reference, state2, utils.queryWrapper);
  const referrers = [];
  const submissions = createSignal([]);
  const matches = createMemo(() => {
    if (typeof options.transformUrl === "function") {
      return getRouteMatches(branches(), options.transformUrl(location.pathname));
    }
    return getRouteMatches(branches(), location.pathname);
  });
  const buildParams = () => {
    const m2 = matches();
    const params2 = {};
    for (let i = 0; i < m2.length; i++) {
      Object.assign(params2, m2[i].params);
    }
    return params2;
  };
  const params = utils.paramsWrapper ? utils.paramsWrapper(buildParams, branches) : createMemoObject(buildParams);
  const baseRoute = {
    pattern: basePath,
    path: () => basePath,
    outlet: () => null,
    resolvePath(to) {
      return resolvePath(basePath, to);
    }
  };
  createRenderEffect(on(source, (source2) => transition("native", source2), { defer: true }));
  return {
    base: baseRoute,
    location,
    params,
    isRouting,
    renderPath,
    parsePath,
    navigatorFactory,
    matches,
    beforeLeave,
    preloadRoute,
    singleFlight: options.singleFlight === void 0 ? true : options.singleFlight,
    submissions
  };
  function navigateFromRoute(route, to, options2) {
    untrack(() => {
      if (typeof to === "number") {
        if (!to) {
        } else if (utils.go) {
          utils.go(to);
        } else {
          console.warn("Router integration does not support relative routing");
        }
        return;
      }
      const queryOnly = !to || to[0] === "?";
      const { replace, resolve, scroll, state: nextState } = {
        replace: false,
        resolve: !queryOnly,
        scroll: true,
        ...options2
      };
      const resolvedTo = resolve ? route.resolvePath(to) : resolvePath(queryOnly && location.pathname || "", to);
      if (resolvedTo === void 0) {
        throw new Error(`Path '${to}' is not a routable path`);
      } else if (referrers.length >= MAX_REDIRECTS) {
        throw new Error("Too many redirects");
      }
      const current = reference();
      if (resolvedTo !== current || nextState !== state2()) {
        if (isServer) ;
        else if (beforeLeave.confirm(resolvedTo, options2)) {
          referrers.push({ value: current, replace, scroll, state: state2() });
          transition("navigate", {
            value: resolvedTo,
            state: nextState
          });
        }
      }
    });
  }
  function navigatorFactory(route) {
    route = route || useContext(RouteContextObj) || baseRoute;
    return (to, options2) => navigateFromRoute(route, to, options2);
  }
  function navigateEnd(next) {
    const first = referrers[0];
    if (first) {
      setSource({
        ...next,
        replace: first.replace,
        scroll: first.scroll
      });
      referrers.length = 0;
    }
  }
  function preloadRoute(url, preloadData) {
    const matches2 = getRouteMatches(branches(), url.pathname);
    const prevIntent = intent;
    intent = "preload";
    for (let match in matches2) {
      const { route, params: params2 } = matches2[match];
      route.component && route.component.preload && route.component.preload();
      const { preload: preload2 } = route;
      preloadData && preload2 && runWithOwner(getContext(), () => preload2({
        params: params2,
        location: {
          pathname: url.pathname,
          search: url.search,
          hash: url.hash,
          query: extractSearchParams(url),
          state: null,
          key: ""
        },
        intent: "preload"
      }));
    }
    intent = prevIntent;
  }
}
function createRouteContext(router, parent, outlet, match) {
  const { base, location, params } = router;
  const { pattern, component, preload: preload2 } = match().route;
  const path = createMemo(() => match().path);
  component && component.preload && component.preload();
  const data = preload2 ? preload2({ params, location, intent: intent || "initial" }) : void 0;
  const route = {
    parent,
    pattern,
    path,
    outlet: () => component ? createComponent(component, {
      params,
      location,
      data,
      get children() {
        return outlet();
      }
    }) : outlet(),
    resolvePath(to) {
      return resolvePath(base.path(), to, path());
    }
  };
  return route;
}
const createRouterComponent = (router) => (props) => {
  const {
    base
  } = props;
  const routeDefs = children(() => props.children);
  const branches = createMemo(() => createBranches(routeDefs(), props.base || ""));
  let context;
  const routerState = createRouterContext(router, branches, () => context, {
    base,
    singleFlight: props.singleFlight,
    transformUrl: props.transformUrl
  });
  router.create && router.create(routerState);
  return createComponent(RouterContextObj.Provider, {
    value: routerState,
    get children() {
      return createComponent(Root, {
        routerState,
        get root() {
          return props.root;
        },
        get preload() {
          return props.rootPreload || props.rootLoad;
        },
        get children() {
          return [memo(() => (context = getOwner()) && null), createComponent(Routes, {
            routerState,
            get branches() {
              return branches();
            }
          })];
        }
      });
    }
  });
};
function Root(props) {
  const location = props.routerState.location;
  const params = props.routerState.params;
  const data = createMemo(() => props.preload && untrack(() => {
    setInPreloadFn(true);
    props.preload({
      params,
      location,
      intent: getIntent() || "initial"
    });
    setInPreloadFn(false);
  }));
  return createComponent(Show, {
    get when() {
      return props.root;
    },
    keyed: true,
    get fallback() {
      return props.children;
    },
    children: (Root2) => createComponent(Root2, {
      params,
      location,
      get data() {
        return data();
      },
      get children() {
        return props.children;
      }
    })
  });
}
function Routes(props) {
  const disposers = [];
  let root2;
  const routeStates = createMemo(on(props.routerState.matches, (nextMatches, prevMatches, prev) => {
    let equal = prevMatches && nextMatches.length === prevMatches.length;
    const next = [];
    for (let i = 0, len = nextMatches.length; i < len; i++) {
      const prevMatch = prevMatches && prevMatches[i];
      const nextMatch = nextMatches[i];
      if (prev && prevMatch && nextMatch.route.key === prevMatch.route.key) {
        next[i] = prev[i];
      } else {
        equal = false;
        if (disposers[i]) {
          disposers[i]();
        }
        createRoot((dispose2) => {
          disposers[i] = dispose2;
          next[i] = createRouteContext(props.routerState, next[i - 1] || props.routerState.base, createOutlet(() => routeStates()[i + 1]), () => props.routerState.matches()[i]);
        });
      }
    }
    disposers.splice(nextMatches.length).forEach((dispose2) => dispose2());
    if (prev && equal) {
      return prev;
    }
    root2 = next[0];
    return next;
  }));
  return createOutlet(() => routeStates() && root2)();
}
const createOutlet = (child) => {
  return () => createComponent(Show, {
    get when() {
      return child();
    },
    keyed: true,
    children: (child2) => createComponent(RouteContextObj.Provider, {
      value: child2,
      get children() {
        return child2.outlet();
      }
    })
  });
};
const Route = (props) => {
  const childRoutes = children(() => props.children);
  return mergeProps(props, {
    get children() {
      return childRoutes();
    }
  });
};
function intercept([value, setValue], get, set) {
  return [value, set ? (v2) => setValue(set(v2)) : setValue];
}
function createRouter(config) {
  let ignore = false;
  const wrap = (value) => typeof value === "string" ? { value } : value;
  const signal = intercept(createSignal(wrap(config.get()), {
    equals: (a, b2) => a.value === b2.value && a.state === b2.state
  }), void 0, (next) => {
    !ignore && config.set(next);
    return next;
  });
  config.init && onCleanup(config.init((value = config.get()) => {
    ignore = true;
    signal[1](wrap(value));
    ignore = false;
  }));
  return createRouterComponent({
    signal,
    create: config.create,
    utils: config.utils
  });
}
function bindEvent(target, type, handler) {
  target.addEventListener(type, handler);
  return () => target.removeEventListener(type, handler);
}
function scrollToHash(hash, fallbackTop) {
  const el = hash && document.getElementById(hash);
  if (el) {
    el.scrollIntoView();
  } else if (fallbackTop) {
    window.scrollTo(0, 0);
  }
}
const actions = /* @__PURE__ */ new Map();
function setupNativeEvents(preload2 = true, explicitLinks = false, actionBase = "/_server", transformUrl) {
  return (router) => {
    const basePath = router.base.path();
    const navigateFromRoute = router.navigatorFactory(router.base);
    let preloadTimeout;
    let lastElement;
    function isSvg(el) {
      return el.namespaceURI === "http://www.w3.org/2000/svg";
    }
    function handleAnchor(evt) {
      if (evt.defaultPrevented || evt.button !== 0 || evt.metaKey || evt.altKey || evt.ctrlKey || evt.shiftKey)
        return;
      const a = evt.composedPath().find((el) => el instanceof Node && el.nodeName.toUpperCase() === "A");
      if (!a || explicitLinks && !a.hasAttribute("link"))
        return;
      const svg2 = isSvg(a);
      const href = svg2 ? a.href.baseVal : a.href;
      const target = svg2 ? a.target.baseVal : a.target;
      if (target || !href && !a.hasAttribute("state"))
        return;
      const rel = (a.getAttribute("rel") || "").split(/\s+/);
      if (a.hasAttribute("download") || rel && rel.includes("external"))
        return;
      const url = svg2 ? new URL(href, document.baseURI) : new URL(href);
      if (url.origin !== window.location.origin || basePath && url.pathname && !url.pathname.toLowerCase().startsWith(basePath.toLowerCase()))
        return;
      return [a, url];
    }
    function handleAnchorClick(evt) {
      const res = handleAnchor(evt);
      if (!res)
        return;
      const [a, url] = res;
      const to = router.parsePath(url.pathname + url.search + url.hash);
      const state2 = a.getAttribute("state");
      evt.preventDefault();
      navigateFromRoute(to, {
        resolve: false,
        replace: a.hasAttribute("replace"),
        scroll: !a.hasAttribute("noscroll"),
        state: state2 ? JSON.parse(state2) : void 0
      });
    }
    function handleAnchorPreload(evt) {
      const res = handleAnchor(evt);
      if (!res)
        return;
      const [a, url] = res;
      transformUrl && (url.pathname = transformUrl(url.pathname));
      router.preloadRoute(url, a.getAttribute("preload") !== "false");
    }
    function handleAnchorMove(evt) {
      clearTimeout(preloadTimeout);
      const res = handleAnchor(evt);
      if (!res)
        return lastElement = null;
      const [a, url] = res;
      if (lastElement === a)
        return;
      transformUrl && (url.pathname = transformUrl(url.pathname));
      preloadTimeout = setTimeout(() => {
        router.preloadRoute(url, a.getAttribute("preload") !== "false");
        lastElement = a;
      }, 20);
    }
    function handleFormSubmit(evt) {
      if (evt.defaultPrevented)
        return;
      let actionRef = evt.submitter && evt.submitter.hasAttribute("formaction") ? evt.submitter.getAttribute("formaction") : evt.target.getAttribute("action");
      if (!actionRef)
        return;
      if (!actionRef.startsWith("https://action/")) {
        const url = new URL(actionRef, mockBase);
        actionRef = router.parsePath(url.pathname + url.search);
        if (!actionRef.startsWith(actionBase))
          return;
      }
      if (evt.target.method.toUpperCase() !== "POST")
        throw new Error("Only POST forms are supported for Actions");
      const handler = actions.get(actionRef);
      if (handler) {
        evt.preventDefault();
        const data = new FormData(evt.target, evt.submitter);
        handler.call({ r: router, f: evt.target }, evt.target.enctype === "multipart/form-data" ? data : new URLSearchParams(data));
      }
    }
    delegateEvents(["click", "submit"]);
    document.addEventListener("click", handleAnchorClick);
    if (preload2) {
      document.addEventListener("mousemove", handleAnchorMove, { passive: true });
      document.addEventListener("focusin", handleAnchorPreload, { passive: true });
      document.addEventListener("touchstart", handleAnchorPreload, { passive: true });
    }
    document.addEventListener("submit", handleFormSubmit);
    onCleanup(() => {
      document.removeEventListener("click", handleAnchorClick);
      if (preload2) {
        document.removeEventListener("mousemove", handleAnchorMove);
        document.removeEventListener("focusin", handleAnchorPreload);
        document.removeEventListener("touchstart", handleAnchorPreload);
      }
      document.removeEventListener("submit", handleFormSubmit);
    });
  };
}
function Router(props) {
  const getSource = () => {
    const url = window.location.pathname.replace(/^\/+/, "/") + window.location.search;
    const state2 = window.history.state && window.history.state._depth && Object.keys(window.history.state).length === 1 ? void 0 : window.history.state;
    return {
      value: url + window.location.hash,
      state: state2
    };
  };
  const beforeLeave = createBeforeLeave();
  return createRouter({
    get: getSource,
    set({ value, replace, scroll, state: state2 }) {
      if (replace) {
        window.history.replaceState(keepDepth(state2), "", value);
      } else {
        window.history.pushState(state2, "", value);
      }
      scrollToHash(decodeURIComponent(window.location.hash.slice(1)), scroll);
      saveCurrentDepth();
    },
    init: (notify) => bindEvent(window, "popstate", notifyIfNotBlocked(notify, (delta) => {
      if (delta && delta < 0) {
        return !beforeLeave.confirm(delta);
      } else {
        const s = getSource();
        return !beforeLeave.confirm(s.value, { state: s.state });
      }
    })),
    create: setupNativeEvents(props.preload, props.explicitLinks, props.actionBase, props.transformUrl),
    utils: {
      go: (delta) => window.history.go(delta),
      beforeLeave
    }
  })(props);
}
var _tmpl$$b = /* @__PURE__ */ template(`<a>`);
function A$1(props) {
  props = mergeProps({
    inactiveClass: "inactive",
    activeClass: "active"
  }, props);
  const [, rest] = splitProps(props, ["href", "state", "class", "activeClass", "inactiveClass", "end"]);
  const to = useResolvedPath(() => props.href);
  const href = useHref(to);
  const location = useLocation();
  const isActive = createMemo(() => {
    const to_ = to();
    if (to_ === void 0) return [false, false];
    const path = normalizePath(to_.split(/[?#]/, 1)[0]).toLowerCase();
    const loc = decodeURI(normalizePath(location.pathname).toLowerCase());
    return [props.end ? path === loc : loc.startsWith(path + "/") || loc === path, path === loc];
  });
  return (() => {
    var _el$ = _tmpl$$b();
    spread(_el$, mergeProps(rest, {
      get href() {
        return href() || props.href;
      },
      get state() {
        return JSON.stringify(props.state);
      },
      get classList() {
        return {
          ...props.class && {
            [props.class]: true
          },
          [props.inactiveClass]: !isActive()[0],
          [props.activeClass]: isActive()[0],
          ...rest.classList
        };
      },
      "link": "",
      get ["aria-current"]() {
        return isActive()[1] ? "page" : void 0;
      }
    }), false);
    return _el$;
  })();
}
const scriptRel = "modulepreload";
const assetsURL = function(dep) {
  return "/" + dep;
};
const seen = {};
const __vitePreload = function preload(baseModule, deps, importerUrl) {
  let promise = Promise.resolve();
  if (deps && deps.length > 0) {
    document.getElementsByTagName("link");
    const cspNonceMeta = document.querySelector(
      "meta[property=csp-nonce]"
    );
    const cspNonce = cspNonceMeta?.nonce || cspNonceMeta?.getAttribute("nonce");
    promise = Promise.allSettled(
      deps.map((dep) => {
        dep = assetsURL(dep);
        if (dep in seen) return;
        seen[dep] = true;
        const isCss = dep.endsWith(".css");
        const cssSelector = isCss ? '[rel="stylesheet"]' : "";
        if (document.querySelector(`link[href="${dep}"]${cssSelector}`)) {
          return;
        }
        const link = document.createElement("link");
        link.rel = isCss ? "stylesheet" : scriptRel;
        if (!isCss) {
          link.as = "script";
        }
        link.crossOrigin = "";
        link.href = dep;
        if (cspNonce) {
          link.setAttribute("nonce", cspNonce);
        }
        document.head.appendChild(link);
        if (isCss) {
          return new Promise((res, rej) => {
            link.addEventListener("load", res);
            link.addEventListener(
              "error",
              () => rej(new Error(`Unable to preload CSS for ${dep}`))
            );
          });
        }
      })
    );
  }
  function handlePreloadError(err) {
    const e = new Event("vite:preloadError", {
      cancelable: true
    });
    e.payload = err;
    window.dispatchEvent(e);
    if (!e.defaultPrevented) {
      throw err;
    }
  }
  return promise.then((res) => {
    for (const item of res || []) {
      if (item.status !== "rejected") continue;
      handlePreloadError(item.reason);
    }
    return baseModule().catch(handlePreloadError);
  });
};
function __classPrivateFieldGet(receiver, state2, kind, f) {
  if (typeof state2 === "function" ? receiver !== state2 || !f : !state2.has(receiver)) throw new TypeError("Cannot read private member from an object whose class did not declare it");
  return kind === "m" ? f : kind === "a" ? f.call(receiver) : f ? f.value : state2.get(receiver);
}
function __classPrivateFieldSet(receiver, state2, value, kind, f) {
  if (typeof state2 === "function" ? receiver !== state2 || true : !state2.has(receiver)) throw new TypeError("Cannot write private member to an object whose class did not declare it");
  return state2.set(receiver, value), value;
}
typeof SuppressedError === "function" ? SuppressedError : function(error, suppressed, message) {
  var e = new Error(message);
  return e.name = "SuppressedError", e.error = error, e.suppressed = suppressed, e;
};
var _Channel_onmessage, _Channel_nextMessageIndex, _Channel_pendingMessages, _Channel_messageEndIndex, _Resource_rid;
const SERIALIZE_TO_IPC_FN = "__TAURI_TO_IPC_KEY__";
function transformCallback(callback, once = false) {
  return window.__TAURI_INTERNALS__.transformCallback(callback, once);
}
class Channel {
  constructor(onmessage) {
    _Channel_onmessage.set(this, void 0);
    _Channel_nextMessageIndex.set(this, 0);
    _Channel_pendingMessages.set(this, []);
    _Channel_messageEndIndex.set(this, void 0);
    __classPrivateFieldSet(this, _Channel_onmessage, onmessage || (() => {
    }));
    this.id = transformCallback((rawMessage) => {
      const index = rawMessage.index;
      if ("end" in rawMessage) {
        if (index == __classPrivateFieldGet(this, _Channel_nextMessageIndex, "f")) {
          this.cleanupCallback();
        } else {
          __classPrivateFieldSet(this, _Channel_messageEndIndex, index);
        }
        return;
      }
      const message = rawMessage.message;
      if (index == __classPrivateFieldGet(this, _Channel_nextMessageIndex, "f")) {
        __classPrivateFieldGet(this, _Channel_onmessage, "f").call(this, message);
        __classPrivateFieldSet(this, _Channel_nextMessageIndex, __classPrivateFieldGet(this, _Channel_nextMessageIndex, "f") + 1);
        while (__classPrivateFieldGet(this, _Channel_nextMessageIndex, "f") in __classPrivateFieldGet(this, _Channel_pendingMessages, "f")) {
          const message2 = __classPrivateFieldGet(this, _Channel_pendingMessages, "f")[__classPrivateFieldGet(this, _Channel_nextMessageIndex, "f")];
          __classPrivateFieldGet(this, _Channel_onmessage, "f").call(this, message2);
          delete __classPrivateFieldGet(this, _Channel_pendingMessages, "f")[__classPrivateFieldGet(this, _Channel_nextMessageIndex, "f")];
          __classPrivateFieldSet(this, _Channel_nextMessageIndex, __classPrivateFieldGet(this, _Channel_nextMessageIndex, "f") + 1);
        }
        if (__classPrivateFieldGet(this, _Channel_nextMessageIndex, "f") === __classPrivateFieldGet(this, _Channel_messageEndIndex, "f")) {
          this.cleanupCallback();
        }
      } else {
        __classPrivateFieldGet(this, _Channel_pendingMessages, "f")[index] = message;
      }
    });
  }
  cleanupCallback() {
    window.__TAURI_INTERNALS__.unregisterCallback(this.id);
  }
  set onmessage(handler) {
    __classPrivateFieldSet(this, _Channel_onmessage, handler);
  }
  get onmessage() {
    return __classPrivateFieldGet(this, _Channel_onmessage, "f");
  }
  [(_Channel_onmessage = /* @__PURE__ */ new WeakMap(), _Channel_nextMessageIndex = /* @__PURE__ */ new WeakMap(), _Channel_pendingMessages = /* @__PURE__ */ new WeakMap(), _Channel_messageEndIndex = /* @__PURE__ */ new WeakMap(), SERIALIZE_TO_IPC_FN)]() {
    return `__CHANNEL__:${this.id}`;
  }
  toJSON() {
    return this[SERIALIZE_TO_IPC_FN]();
  }
}
async function invoke(cmd, args = {}, options) {
  return window.__TAURI_INTERNALS__.invoke(cmd, args, options);
}
class Resource {
  get rid() {
    return __classPrivateFieldGet(this, _Resource_rid, "f");
  }
  constructor(rid) {
    _Resource_rid.set(this, void 0);
    __classPrivateFieldSet(this, _Resource_rid, rid);
  }
  /**
   * Destroys and cleans up this resource from memory.
   * **You should not call any method on this object anymore and should drop any reference to it.**
   */
  async close() {
    return invoke("plugin:resources|close", {
      rid: this.rid
    });
  }
}
_Resource_rid = /* @__PURE__ */ new WeakMap();
const RECORD_TYPES = [
  { type: "login", label: "Logins", blurb: "Services, sites & apps", icon: "🔑", category: "identity", group: "Identity & Access" },
  { type: "identification", label: "IDs", blurb: "Government & institutional IDs", icon: "🆔", category: "identity", group: "Identity & Access" },
  { type: "contact", label: "Contacts", blurb: "People", icon: "👤", category: "identity", group: "Identity & Access" },
  { type: "bank_account", label: "Bank accounts", blurb: "Checking, savings, etc.", icon: "🏦", category: "money", group: "Money" },
  { type: "credit_card", label: "Credit cards", blurb: "Issuer / network / last 4", icon: "💳", category: "money", group: "Money" },
  { type: "investment", label: "Investments", blurb: "Brokerage, 401k, IRA", icon: "📈", category: "money", group: "Money" },
  { type: "income_source", label: "Income", blurb: "Jobs, contracts, gigs", icon: "💼", category: "money", group: "Money" },
  { type: "subscription", label: "Subscriptions", blurb: "Recurring services", icon: "🔁", category: "money", group: "Money" },
  { type: "insurance", label: "Insurance", blurb: "Policies & claims", icon: "🛡", category: "money", group: "Money" },
  { type: "vehicle", label: "Vehicles", blurb: "Cars, plates, VINs", icon: "🚗", category: "property", group: "Property" },
  { type: "residence", label: "Residences", blurb: "Rentals, leases, addresses", icon: "🏠", category: "property", group: "Property" },
  { type: "phone", label: "Phones", blurb: "Devices & lines", icon: "📱", category: "property", group: "Property" },
  { type: "address", label: "Addresses", blurb: "Anywhere important", icon: "📍", category: "property", group: "Property" },
  { type: "document", label: "Documents", blurb: "Passports, leases, etc.", icon: "📄", category: "documents", group: "Documents & Records" },
  { type: "infrastructure", label: "Infrastructure", blurb: "Servers, services, decisions", icon: "🖥", category: "documents", group: "Documents & Records" },
  { type: "domain", label: "Domains", blurb: "DNS records", icon: "🌐", category: "documents", group: "Documents & Records" },
  { type: "runbook", label: "Runbooks", blurb: "Step-by-step procedures", icon: "📋", category: "notes", group: "Notes & Logs" },
  { type: "work_log", label: "Work logs", blurb: "Dated activity", icon: "🗒", category: "notes", group: "Notes & Logs" },
  { type: "note", label: "Notes", blurb: "Free-form markdown", icon: "✏", category: "notes", group: "Notes & Logs" },
  { type: "health", label: "Health", blurb: "Providers, allergies, history", icon: "❤", category: "notes", group: "Notes & Logs" }
];
const META_BY_TYPE = Object.fromEntries(
  RECORD_TYPES.map((t) => [t.type, t])
);
const api = {
  defaultPath: () => invoke("default_path"),
  status: () => invoke("status"),
  listUsers: (path) => invoke("list_users", { path: path ?? null }),
  init: (username, password, path) => invoke("init", { username, password, path: path ?? null }),
  unlock: (username, password, path) => invoke("unlock", { username, password, path: path ?? null }),
  lock: () => invoke("lock"),
  addUser: (username, password) => invoke("add_user", { username, password }),
  removeUser: (username) => invoke("remove_user", { username }),
  changePassword: (newPassword) => invoke("change_password", { newPassword }),
  listRecords: (type) => invoke("list_records", { type }),
  showRecord: (id, reveal = false) => invoke("show_record", { id, reveal }),
  addRecord: (type, fields) => invoke("add_record", { type, fields }),
  updateRecord: (id, fields) => invoke("update_record", { id, fields }),
  deleteRecord: (id) => invoke("delete_record", { id }),
  find: (query) => invoke("find", { query }),
  audit: (verify) => invoke("audit", { verify }),
  configureSync: (baseUrl) => invoke("configure_sync", { baseUrl }),
  recordTitles: () => invoke("record_titles"),
  rewriteAuditChain: () => invoke("rewrite_audit_chain"),
  syncPush: (serverUrl, vaultId) => invoke("sync_push", { serverUrl, vaultId }),
  syncPull: (serverUrl, vaultId) => invoke("sync_pull", { serverUrl, vaultId }),
  setupSharedSync: (vaultId, passphrase, serverUrl) => invoke("setup_shared_sync", {
    vaultId,
    passphrase,
    serverUrl: serverUrl ?? null
  }),
  revealSharedSync: (vaultId) => invoke("reveal_shared_sync", { vaultId }),
  deleteSharedSync: (vaultId) => invoke("delete_shared_sync", { vaultId }),
  listSharedSyncs: () => invoke("list_shared_syncs"),
  autoSyncStatus: () => invoke("auto_sync_status"),
  setAutoSync: (enabled) => invoke("set_auto_sync", { enabled }),
  exportBundle: (passphrase) => invoke("export_bundle", { passphrase }),
  importBundle: (bytes, passphrase) => invoke("import_bundle", { bytes, passphrase }),
  importToNewVault: (params) => invoke("import_to_new_vault", params),
  recoverFromSync: (params) => invoke("recover_from_sync", params)
};
const [status, setStatus] = createSignal({
  unlocked: false,
  username: null
});
const [vaultPath, setVaultPath] = createSignal("");
const [users, setUsers] = createSignal([]);
function persistedSignal(key, initial) {
  let starting = initial;
  try {
    if (typeof localStorage !== "undefined") {
      const stored = localStorage.getItem(key);
      if (stored !== null) starting = stored;
    }
  } catch {
  }
  const [get, set] = createSignal(starting, { equals: false });
  return [
    get,
    (v2) => {
      set(() => v2);
      try {
        if (typeof localStorage !== "undefined") {
          localStorage.setItem(key, v2);
        }
      } catch {
      }
    }
  ];
}
const [syncUrl, setSyncUrl] = persistedSignal("keepsake.syncUrl", "");
const [syncVaultId, setSyncVaultId] = persistedSignal(
  "keepsake.syncVaultId",
  ""
);
const [toast, setToast] = createSignal(null);
function showToast(kind, text2) {
  setToast({ kind, text: text2 });
  setTimeout(() => setToast(null), 3500);
}
async function refreshStatus() {
  try {
    setStatus(await api.status());
  } catch {
    setStatus({ unlocked: false, username: null });
  }
}
async function refreshUsers() {
  try {
    const p = await api.defaultPath();
    setVaultPath(p);
    const u = await api.listUsers(p);
    setUsers(u);
  } catch {
    setUsers([]);
  }
}
const state = {
  status,
  setStatus,
  vaultPath,
  setVaultPath,
  users,
  setUsers,
  syncUrl,
  setSyncUrl,
  syncVaultId,
  setSyncVaultId,
  toast,
  setToast
};
var _tmpl$$a = /* @__PURE__ */ template(`<div>`);
function Toast(props) {
  return (() => {
    var _el$ = _tmpl$$a();
    insert(_el$, () => props.text);
    createRenderEffect(() => className(_el$, `toast ${props.kind}`));
    return _el$;
  })();
}
function toRecord(payload) {
  if (payload && typeof payload === "object" && !Array.isArray(payload)) {
    return payload;
  }
  return { type: "" };
}
function daysUntil(iso) {
  if (!iso) return null;
  const d2 = new Date(iso);
  if (isNaN(d2.getTime())) return null;
  const ms = d2.getTime() - Date.now();
  return Math.floor(ms / (24 * 60 * 60 * 1e3));
}
function fmtDate(iso) {
  if (!iso) return "?";
  const d2 = new Date(iso);
  if (isNaN(d2.getTime())) return "?";
  return d2.toLocaleDateString();
}
async function generateInsights() {
  const out = [];
  const [
    identifications,
    documents,
    insurances,
    bankAccounts,
    creditCards,
    vehicles,
    logins,
    notes,
    health,
    runbooks,
    workLogs
  ] = await Promise.all([
    list("identification"),
    list("document"),
    list("insurance"),
    list("bank_account"),
    list("credit_card"),
    list("vehicle"),
    list("login"),
    list("note"),
    list("health"),
    list("runbook"),
    list("work_log")
  ]);
  for (const e of identifications) {
    const r = await fetchRecord(e.id);
    if (!r) continue;
    const d2 = daysUntil(r.expires_on);
    if (d2 !== null && d2 <= 60) {
      out.push({
        id: `id-exp-${e.id}`,
        severity: d2 < 0 ? "warn" : d2 < 14 ? "warn" : "info",
        title: d2 < 0 ? `${r.id_type} expired ${-d2} day${-d2 === 1 ? "" : "s"} ago` : `${r.id_type} expires in ${d2} day${d2 === 1 ? "" : "s"}`,
        detail: `${r.holder ?? "?"} — expires ${fmtDate(r.expires_on)}`,
        to: `/r/${e.id}`
      });
    }
  }
  for (const e of documents) {
    const r = await fetchRecord(e.id);
    if (!r) continue;
    const d2 = daysUntil(r.expires_on);
    if (d2 !== null && d2 <= 60) {
      out.push({
        id: `doc-exp-${e.id}`,
        severity: d2 < 14 ? "warn" : "info",
        title: d2 < 0 ? `${r.document_type} expired ${-d2} day${-d2 === 1 ? "" : "s"} ago` : `${r.document_type} expires in ${d2} day${d2 === 1 ? "" : "s"}`,
        detail: `${r.title} — expires ${fmtDate(r.expires_on)}`,
        to: `/r/${e.id}`
      });
    }
  }
  for (const e of insurances) {
    const r = await fetchRecord(e.id);
    if (!r) continue;
    const d2 = daysUntil(r.renewal_on);
    if (d2 !== null && d2 <= 60) {
      out.push({
        id: `ins-ren-${e.id}`,
        severity: d2 < 14 ? "warn" : "info",
        title: d2 < 0 ? `${r.policy_type} renewal overdue (${-d2}d)` : `${r.policy_type} renews in ${d2} day${d2 === 1 ? "" : "s"}`,
        detail: `${r.provider} — ${fmtDate(r.renewal_on)}`,
        to: `/r/${e.id}`
      });
    }
  }
  for (const e of creditCards) {
    const r = await fetchRecord(e.id);
    if (!r) continue;
    const d2 = daysUntil(parseExpiration(r.expiration));
    if (d2 !== null && d2 <= 60) {
      out.push({
        id: `cc-exp-${e.id}`,
        severity: d2 < 14 ? "warn" : "info",
        title: d2 < 0 ? `Card expired ${-d2} day${-d2 === 1 ? "" : "s"} ago` : `Card expires in ${d2} day${d2 === 1 ? "" : "s"}`,
        detail: `${r.issuer} • ${r.network} — ${r.expiration}`,
        to: `/r/${e.id}`
      });
    }
  }
  for (const e of logins) {
    const r = await fetchRecord(e.id);
    if (!r) continue;
    const hasTotp = !!(r.totp_secret && String(r.totp_secret).trim());
    if (!hasTotp) {
      out.push({
        id: `login-nototp-${e.id}`,
        severity: "info",
        title: `${r.service} has no 2FA configured`,
        detail: `${r.username} — add a TOTP secret to harden this account.`,
        to: `/r/${e.id}/edit`
      });
    }
  }
  for (const e of vehicles) {
    const r = await fetchRecord(e.id);
    if (!r) continue;
    const hasVin = !!(r.vin && String(r.vin).trim());
    const hasPlate = !!(r.license_plate && String(r.license_plate).trim());
    if (!hasVin || !hasPlate) {
      const missing = [!hasVin && "VIN", !hasPlate && "plate"].filter(Boolean).join(" & ");
      out.push({
        id: `veh-missing-${e.id}`,
        severity: "info",
        title: `${r.year} ${r.make_model} missing ${missing}`,
        detail: `Fill in the ${missing} for completeness.`,
        to: `/r/${e.id}/edit`
      });
    }
  }
  for (const e of [...insurances, ...bankAccounts, ...vehicles]) {
    const r = await fetchRecord(e.id);
    if (!r) continue;
    const holders = r.holders || r.drivers;
    if (holders && Array.isArray(holders) && holders.length === 0) {
      out.push({
        id: `no-holders-${e.id}`,
        severity: "info",
        title: `${r.type} has no holder assigned`,
        detail: `Add a holder (or "joint") so the audit log is meaningful.`,
        to: `/r/${e.id}/edit`
      });
    }
  }
  const sixMonths = 1e3 * 60 * 60 * 24 * 30 * 6;
  for (const e of notes) {
    const updated = new Date(e.updated_at).getTime();
    if (Date.now() - updated > sixMonths) {
      out.push({
        id: `stale-note-${e.id}`,
        severity: "info",
        title: `Note not updated in a while`,
        detail: `Last updated ${fmtDate(e.updated_at)}. Review and refresh.`,
        to: `/r/${e.id}`
      });
    }
  }
  for (const e of health) {
    const updated = new Date(e.updated_at).getTime();
    if (Date.now() - updated > sixMonths) {
      out.push({
        id: `stale-health-${e.id}`,
        severity: "info",
        title: `Health record not updated in 6+ months`,
        detail: `Last updated ${fmtDate(e.updated_at)}. Verify still accurate.`,
        to: `/r/${e.id}`
      });
    }
  }
  const last30 = 1e3 * 60 * 60 * 24 * 30;
  const recent = workLogs.filter((e) => Date.now() - new Date(e.updated_at).getTime() < last30);
  if (workLogs.length > 0 && recent.length === 0) {
    out.push({
      id: "wl-gap",
      severity: "info",
      title: "No work-log entries in the last 30 days",
      detail: `You have ${workLogs.length} historical entries but nothing recent. Add an entry to keep the timeline current.`,
      to: "/c/work_log/new"
    });
  }
  if (runbooks.length === 0) {
    out.push({
      id: "rb-none",
      severity: "info",
      title: "No runbooks yet",
      detail: `Scenario runbooks (insurance claims, infra incidents, etc.) become invaluable during stress. Create your first one.`,
      to: "/c/runbook/new"
    });
  }
  const byBank = /* @__PURE__ */ new Map();
  for (const e of bankAccounts) {
    const r = await fetchRecord(e.id);
    if (!r) continue;
    const key = `${r.bank?.toLowerCase()}|${r.account_type?.toLowerCase()}`;
    const list2 = byBank.get(key) ?? [];
    list2.push(e);
    byBank.set(key, list2);
  }
  for (const [key, entries2] of byBank) {
    if (entries2.length > 1) {
      out.push({
        id: `dup-bank-${key}`,
        severity: "info",
        title: `Possible duplicate bank account`,
        detail: `${entries2.length} records at the same bank with the same type. Check if they should be merged.`,
        to: `/c/bank_account`
      });
    }
  }
  const total = identifications.length + documents.length + insurances.length + creditCards.length + vehicles.length + logins.length + notes.length;
  if (total === 0) {
    out.push({
      id: "welcome",
      severity: "ok",
      title: "Welcome to Keepsake",
      detail: "Start by adding a login, a document, or a bank account. Use the sidebar to navigate categories."
    });
  } else if (total < 5) {
    out.push({
      id: "early",
      severity: "info",
      title: "You're getting started",
      detail: `${total} record${total === 1 ? "" : "s"} so far. Consider setting up an export to back up your vault.`,
      to: "/settings#export"
    });
  }
  const order = { warn: 0, info: 1, ok: 2 };
  out.sort((a, b2) => order[a.severity] - order[b2.severity]);
  return out;
}
async function list(type) {
  try {
    return await api.listRecords(type);
  } catch {
    return [];
  }
}
async function fetchRecord(id) {
  try {
    const r = await api.showRecord(id, true);
    return toRecord(r);
  } catch {
    return null;
  }
}
function parseExpiration(s) {
  if (!s) return null;
  if (/^\d{4}-\d{2}-\d{2}/.test(s)) return s;
  const m2 = s.match(/^(\d{1,2})\/(\d{2,4})$/);
  if (m2) {
    const month = parseInt(m2[1], 10);
    let year = parseInt(m2[2], 10);
    if (year < 100) year += 2e3;
    const d2 = new Date(Date.UTC(year, month, 0));
    return d2.toISOString();
  }
  return null;
}
let installed = false;
let navigateFn = null;
function installLinkClickHandler(navigate) {
  if (installed) return;
  installed = true;
  navigateFn = navigate;
  document.addEventListener("click", (ev) => {
    const target = ev.target;
    if (!(target instanceof Element)) return;
    const a = target.closest("a.keepsake-link");
    if (!a) return;
    const href = a.getAttribute("href");
    const uuid = a.getAttribute("data-uuid");
    if (!href || !uuid) return;
    ev.preventDefault();
    ev.stopPropagation();
    const path = href.replace(/^#/, "");
    navigateFn?.(path);
  });
}
var _tmpl$$9 = /* @__PURE__ */ template(`<div class=app>`), _tmpl$2$8 = /* @__PURE__ */ template(`<form class=unlock-form><div class=unlock-field><label>username</label><input autocomplete=username required autofocus></div><div class=unlock-field><label>password</label><input type=password autocomplete=current-password required></div><div class=unlock-actions><button type=submit class="btn btn-primary unlock-flex">`), _tmpl$3$8 = /* @__PURE__ */ template(`<div class=form-error>`), _tmpl$4$8 = /* @__PURE__ */ template(`<div class=unlock-form><button type=button class="btn btn-primary btn-block btn-lg">+ Create new vault</button><form id=unlock-init-form class="unlock-init-form hidden"><div class=unlock-field><label>username</label><input autocomplete=username required></div><div class=unlock-field><label>password</label><input type=password autocomplete=new-password required></div><div class=unlock-actions><button type=submit class="btn btn-primary btn-block"></button></div></form><div class=unlock-divider><span>or</span></div><button type=button class="btn btn-block btn-lg">⤓ Import .ksk bundle</button><form id=unlock-import-form class="unlock-init-form hidden"><div class=unlock-field><label>bundle file</label><div class=row><input type=text placeholder="No file selected"readonly><button type=button class=btn>Choose…</button></div></div><div class=unlock-field><label>export passphrase</label><input type=password autocomplete=current-password required></div><div class=unlock-actions><button type=submit class="btn btn-primary btn-block"></button></div></form><div class=unlock-divider><span>or</span></div><button type=button class="btn btn-block btn-lg">⇄ Recover from sync</button><form id=unlock-recover-form class="unlock-init-form hidden"><div class=unlock-field><label>server URL</label><input type=url placeholder=https://sync.example.com required></div><div class=unlock-field><label>vault id</label><input type=text placeholder=family required></div><div class=unlock-field><label>sync passphrase</label><input type=password autocomplete=current-password required></div><div class=unlock-divider><span>local account on this device</span></div><div class=unlock-field><label>username</label><input autocomplete=username required></div><div class=unlock-field><label>password</label><input type=password autocomplete=new-password required></div><div class=unlock-actions><button type=submit class="btn btn-primary btn-block">`), _tmpl$5$8 = /* @__PURE__ */ template(`<div><span class=muted-small>users on this device:</span> `), _tmpl$6$6 = /* @__PURE__ */ template(`<div class=unlock-shell><div class=unlock-card><div class=unlock-brand><div class=unlock-mark>K</div><span class=unlock-name>Keepsake</span></div><h1 class=unlock-title></h1><p class=unlock-sub></p><div class=unlock-meta><div><span class=muted-small>vault:</span> <code>`), _tmpl$7$6 = /* @__PURE__ */ template(`<code>`), _tmpl$8$5 = /* @__PURE__ */ template(`<div class=shell><main>`), _tmpl$9$3 = /* @__PURE__ */ template(`<aside class=sidebar><div class=sidebar-brand><div class=sidebar-mark>K</div><div class=sidebar-brand-text><span class=sidebar-brand-name>Keepsake</span><span class=sidebar-brand-user></span></div></div><nav class=sidebar-nav></nav><div class=sidebar-scroll><div class=sidebar-section>System</div><nav></nav></div><div class=sidebar-footer><button class="btn btn-ghost"title="Lock vault">🔒 Lock`), _tmpl$0$3 = /* @__PURE__ */ template(`<div class=sidebar-group><div class=sidebar-section></div><nav>`), _tmpl$1$2 = /* @__PURE__ */ template(`<span class=ic>`), _tmpl$10$2 = /* @__PURE__ */ template(`<span class=lbl>`), _tmpl$11$1 = /* @__PURE__ */ template(`<span class=sidebar-badge>`);
function App(props) {
  const navigate = useNavigate();
  onMount(async () => {
    installLinkClickHandler(navigate);
    await refreshUsers();
    await refreshStatus();
  });
  return (() => {
    var _el$ = _tmpl$$9();
    insert(_el$, createComponent(Show, {
      get when() {
        return state.status().unlocked;
      },
      get fallback() {
        return createComponent(Unlock, {});
      },
      get children() {
        return createComponent(Shell, {
          get children() {
            return props.children;
          }
        });
      }
    }), null);
    insert(_el$, createComponent(Show, {
      get when() {
        return state.toast();
      },
      children: (t) => createComponent(Toast, {
        get kind() {
          return t().kind;
        },
        get text() {
          return t().text;
        }
      })
    }), null);
    return _el$;
  })();
}
function Unlock() {
  const [username, setUsername] = createSignal("");
  const [password, setPassword] = createSignal("");
  const [busy, setBusy] = createSignal(false);
  const [importing, setImporting] = createSignal(false);
  const [importPass, setImportPass] = createSignal("");
  const [importBytes, setImportBytes] = createSignal("");
  const [importError, setImportError] = createSignal(null);
  const [recovering, setRecovering] = createSignal(false);
  const [recoverServerUrl, setRecoverServerUrl] = createSignal("");
  const [recoverVaultId, setRecoverVaultId] = createSignal("");
  const [recoverPassphrase, setRecoverPassphrase] = createSignal("");
  const hasVault = () => state.users().length > 0;
  onMount(() => {
    void refreshUsers();
  });
  async function submit(e) {
    e.preventDefault();
    setBusy(true);
    try {
      if (hasVault()) {
        await api.unlock(username(), password());
        showToast("ok", `Welcome back, ${username()}`);
      } else {
        await api.init(username(), password());
        showToast("ok", `Vault created for ${username()}`);
      }
      await refreshStatus();
    } catch (e2) {
      showToast("err", String(e2));
    } finally {
      setBusy(false);
    }
  }
  async function pickFile() {
    setImportError(null);
    try {
      const {
        open
      } = await __vitePreload(async () => {
        const {
          open: open2
        } = await import("./index-DuDVuQC0.js");
        return {
          open: open2
        };
      }, true ? [] : void 0);
      const selected = await open({
        multiple: false,
        filters: [{
          name: "Keepsake bundle",
          extensions: ["ksk"]
        }]
      });
      if (!selected) return;
      const {
        readTextFile
      } = await __vitePreload(async () => {
        const {
          readTextFile: readTextFile2
        } = await import("./index-X0YsrTfK.js");
        return {
          readTextFile: readTextFile2
        };
      }, true ? [] : void 0);
      const txt = await readTextFile(selected);
      setImportBytes(txt);
    } catch (e) {
      setImportError(String(e));
    }
  }
  async function doImport(e) {
    e.preventDefault();
    if (!importBytes().trim()) {
      setImportError("Pick a .ksk file first");
      return;
    }
    if (!importPass()) {
      setImportError("Enter the export passphrase");
      return;
    }
    if (!username().trim()) {
      setImportError("Pick a username to associate with the import");
      return;
    }
    if (!password()) {
      setImportError("Pick a password for the new account");
      return;
    }
    let arr;
    try {
      arr = JSON.parse(importBytes().trim());
      if (!Array.isArray(arr)) throw new Error("not a JSON array");
    } catch (err) {
      setImportError(`Invalid bundle format: ${err}`);
      return;
    }
    setImporting(true);
    setImportError(null);
    try {
      await api.importToNewVault({
        bytes: arr,
        passphrase: importPass(),
        username: username(),
        password: password()
      });
      await refreshStatus();
      showToast("ok", "Bundle imported");
      setImportBytes("");
      setImportPass("");
    } catch (err) {
      setImportError(String(err));
    } finally {
      setImporting(false);
    }
  }
  async function doRecover(e) {
    e.preventDefault();
    if (!recoverServerUrl().trim()) {
      setImportError("Server URL is required");
      return;
    }
    if (!recoverVaultId().trim()) {
      setImportError("Vault id is required");
      return;
    }
    if (!recoverPassphrase()) {
      setImportError("Sync passphrase is required");
      return;
    }
    if (!username().trim()) {
      setImportError("Pick a username for the new local account");
      return;
    }
    if (!password()) {
      setImportError("Pick a password for the new local account");
      return;
    }
    setRecovering(true);
    setImportError(null);
    try {
      await api.recoverFromSync({
        serverUrl: recoverServerUrl().trim(),
        vaultId: recoverVaultId().trim(),
        syncPassphrase: recoverPassphrase(),
        username: username(),
        password: password()
      });
      state.setSyncUrl(recoverServerUrl().trim());
      state.setSyncVaultId(recoverVaultId().trim());
      await refreshStatus();
      try {
        const n = await api.syncPull(recoverServerUrl().trim(), recoverVaultId().trim());
        showToast("ok", `Recovered ${n} record(s) from sync`);
      } catch (e2) {
        showToast("err", `recovered, but initial pull failed: ${String(e2)}`);
      }
      setRecoverPassphrase("");
    } catch (err) {
      setImportError(String(err));
    } finally {
      setRecovering(false);
    }
  }
  return (() => {
    var _el$2 = _tmpl$6$6(), _el$3 = _el$2.firstChild, _el$4 = _el$3.firstChild, _el$5 = _el$4.nextSibling, _el$6 = _el$5.nextSibling, _el$61 = _el$6.nextSibling, _el$65 = _el$61.firstChild, _el$66 = _el$65.firstChild, _el$67 = _el$66.nextSibling, _el$68 = _el$67.nextSibling;
    insert(_el$5, (() => {
      var _c$ = memo(() => !!hasVault());
      return () => _c$() ? "Unlock your vault" : importing() ? "Importing…" : "Set up your vault";
    })());
    insert(_el$6, () => hasVault() ? "End-to-end encrypted. Local-first, sync-optional." : "Create a new vault, or import an existing .ksk bundle.");
    insert(_el$3, createComponent(Show, {
      get when() {
        return hasVault();
      },
      get children() {
        var _el$7 = _tmpl$2$8(), _el$8 = _el$7.firstChild, _el$9 = _el$8.firstChild, _el$0 = _el$9.nextSibling, _el$1 = _el$8.nextSibling, _el$10 = _el$1.firstChild, _el$11 = _el$10.nextSibling, _el$12 = _el$1.nextSibling, _el$13 = _el$12.firstChild;
        _el$7.addEventListener("submit", submit);
        _el$0.$$input = (e) => setUsername(e.currentTarget.value);
        _el$11.$$input = (e) => setPassword(e.currentTarget.value);
        insert(_el$13, () => busy() ? "Working…" : "Unlock");
        createRenderEffect(() => _el$13.disabled = busy());
        createRenderEffect(() => _el$0.value = username());
        createRenderEffect(() => _el$11.value = password());
        return _el$7;
      }
    }), _el$61);
    insert(_el$3, createComponent(Show, {
      get when() {
        return !hasVault();
      },
      get children() {
        var _el$14 = _tmpl$4$8(), _el$15 = _el$14.firstChild, _el$16 = _el$15.nextSibling, _el$17 = _el$16.firstChild, _el$18 = _el$17.firstChild, _el$19 = _el$18.nextSibling, _el$20 = _el$17.nextSibling, _el$21 = _el$20.firstChild, _el$22 = _el$21.nextSibling, _el$23 = _el$20.nextSibling, _el$24 = _el$23.firstChild, _el$25 = _el$16.nextSibling, _el$26 = _el$25.nextSibling, _el$27 = _el$26.nextSibling, _el$28 = _el$27.firstChild, _el$29 = _el$28.firstChild, _el$30 = _el$29.nextSibling, _el$31 = _el$30.firstChild, _el$32 = _el$31.nextSibling, _el$33 = _el$28.nextSibling, _el$34 = _el$33.firstChild, _el$35 = _el$34.nextSibling, _el$37 = _el$33.nextSibling, _el$38 = _el$37.firstChild, _el$39 = _el$27.nextSibling, _el$40 = _el$39.nextSibling, _el$41 = _el$40.nextSibling, _el$42 = _el$41.firstChild, _el$43 = _el$42.firstChild, _el$44 = _el$43.nextSibling, _el$45 = _el$42.nextSibling, _el$46 = _el$45.firstChild, _el$47 = _el$46.nextSibling, _el$48 = _el$45.nextSibling, _el$49 = _el$48.firstChild, _el$50 = _el$49.nextSibling, _el$51 = _el$48.nextSibling, _el$52 = _el$51.nextSibling, _el$53 = _el$52.firstChild, _el$54 = _el$53.nextSibling, _el$55 = _el$52.nextSibling, _el$56 = _el$55.firstChild, _el$57 = _el$56.nextSibling, _el$59 = _el$55.nextSibling, _el$60 = _el$59.firstChild;
        _el$15.$$click = () => {
          document.getElementById("unlock-init-form")?.classList.toggle("hidden");
          document.getElementById("unlock-import-form")?.classList.add("hidden");
          document.getElementById("unlock-recover-form")?.classList.add("hidden");
        };
        _el$16.addEventListener("submit", submit);
        _el$19.$$input = (e) => setUsername(e.currentTarget.value);
        _el$22.$$input = (e) => setPassword(e.currentTarget.value);
        insert(_el$24, () => busy() ? "Creating…" : "Create vault");
        _el$26.$$click = () => {
          document.getElementById("unlock-import-form")?.classList.toggle("hidden");
          document.getElementById("unlock-init-form")?.classList.add("hidden");
          document.getElementById("unlock-recover-form")?.classList.add("hidden");
        };
        _el$27.addEventListener("submit", doImport);
        _el$32.$$click = pickFile;
        _el$35.$$input = (e) => setImportPass(e.currentTarget.value);
        insert(_el$27, createComponent(Show, {
          get when() {
            return importError();
          },
          get children() {
            var _el$36 = _tmpl$3$8();
            insert(_el$36, importError);
            return _el$36;
          }
        }), _el$37);
        insert(_el$38, () => importing() ? "Importing…" : "Import bundle");
        _el$40.$$click = () => {
          document.getElementById("unlock-recover-form")?.classList.toggle("hidden");
          document.getElementById("unlock-init-form")?.classList.add("hidden");
          document.getElementById("unlock-import-form")?.classList.add("hidden");
        };
        _el$41.addEventListener("submit", doRecover);
        _el$44.$$input = (e) => setRecoverServerUrl(e.currentTarget.value);
        _el$47.$$input = (e) => setRecoverVaultId(e.currentTarget.value);
        _el$50.$$input = (e) => setRecoverPassphrase(e.currentTarget.value);
        _el$54.$$input = (e) => setUsername(e.currentTarget.value);
        _el$57.$$input = (e) => setPassword(e.currentTarget.value);
        insert(_el$41, createComponent(Show, {
          get when() {
            return importError();
          },
          get children() {
            var _el$58 = _tmpl$3$8();
            insert(_el$58, importError);
            return _el$58;
          }
        }), _el$59);
        insert(_el$60, () => recovering() ? "Recovering…" : "Recover from sync");
        createRenderEffect((_p$) => {
          var _v$ = importing() || recovering(), _v$2 = busy(), _v$3 = importing() || recovering(), _v$4 = importing() || recovering(), _v$5 = importing() || recovering(), _v$6 = importing() || recovering(), _v$7 = recovering() || importing();
          _v$ !== _p$.e && (_el$15.disabled = _p$.e = _v$);
          _v$2 !== _p$.t && (_el$24.disabled = _p$.t = _v$2);
          _v$3 !== _p$.a && (_el$26.disabled = _p$.a = _v$3);
          _v$4 !== _p$.o && (_el$32.disabled = _p$.o = _v$4);
          _v$5 !== _p$.i && (_el$38.disabled = _p$.i = _v$5);
          _v$6 !== _p$.n && (_el$40.disabled = _p$.n = _v$6);
          _v$7 !== _p$.s && (_el$60.disabled = _p$.s = _v$7);
          return _p$;
        }, {
          e: void 0,
          t: void 0,
          a: void 0,
          o: void 0,
          i: void 0,
          n: void 0,
          s: void 0
        });
        createRenderEffect(() => _el$19.value = username());
        createRenderEffect(() => _el$22.value = password());
        createRenderEffect(() => _el$31.value = importBytes() ? "(loaded)" : "");
        createRenderEffect(() => _el$35.value = importPass());
        createRenderEffect(() => _el$44.value = recoverServerUrl());
        createRenderEffect(() => _el$47.value = recoverVaultId());
        createRenderEffect(() => _el$50.value = recoverPassphrase());
        createRenderEffect(() => _el$54.value = username());
        createRenderEffect(() => _el$57.value = password());
        return _el$14;
      }
    }), _el$61);
    insert(_el$61, createComponent(Show, {
      get when() {
        return memo(() => !!hasVault())() && state.users().length > 0;
      },
      get children() {
        var _el$62 = _tmpl$5$8(), _el$63 = _el$62.firstChild;
        _el$63.nextSibling;
        insert(_el$62, () => state.users().map((u, i) => [i > 0 ? ", " : "", (() => {
          var _el$69 = _tmpl$7$6();
          insert(_el$69, u);
          return _el$69;
        })()]), null);
        return _el$62;
      }
    }), _el$65);
    insert(_el$68, () => state.vaultPath() || "—");
    return _el$2;
  })();
}
function Shell(props) {
  return (() => {
    var _el$70 = _tmpl$8$5(), _el$71 = _el$70.firstChild;
    insert(_el$70, createComponent(Sidebar, {}), _el$71);
    insert(_el$71, () => props.children);
    return _el$70;
  })();
}
function Sidebar() {
  const nav = useNavigate();
  async function lockNow() {
    await api.lock();
    await refreshStatus();
    nav("/");
  }
  const groups = [{
    name: "Identity & Access",
    types: RECORD_TYPES.filter((t) => t.group === "Identity & Access")
  }, {
    name: "Money",
    types: RECORD_TYPES.filter((t) => t.group === "Money")
  }, {
    name: "Property",
    types: RECORD_TYPES.filter((t) => t.group === "Property")
  }, {
    name: "Documents & Records",
    types: RECORD_TYPES.filter((t) => t.group === "Documents & Records")
  }, {
    name: "Notes & Logs",
    types: RECORD_TYPES.filter((t) => t.group === "Notes & Logs")
  }];
  return (() => {
    var _el$72 = _tmpl$9$3(), _el$73 = _el$72.firstChild, _el$74 = _el$73.firstChild, _el$75 = _el$74.nextSibling, _el$76 = _el$75.firstChild, _el$77 = _el$76.nextSibling, _el$78 = _el$73.nextSibling, _el$79 = _el$78.nextSibling, _el$80 = _el$79.firstChild, _el$81 = _el$80.nextSibling, _el$82 = _el$79.nextSibling, _el$83 = _el$82.firstChild;
    insert(_el$77, () => state.status().username ?? "—");
    insert(_el$78, createComponent(SidebarLink, {
      href: "/",
      icon: "◐",
      label: "Dashboard",
      end: true
    }));
    insert(_el$79, () => groups.map((g2) => (() => {
      var _el$84 = _tmpl$0$3(), _el$85 = _el$84.firstChild, _el$86 = _el$85.nextSibling;
      insert(_el$85, () => g2.name);
      insert(_el$86, () => g2.types.map((t) => createComponent(SidebarLink, {
        get href() {
          return `/c/${t.type}`;
        },
        get icon() {
          return t.icon;
        },
        get label() {
          return t.label;
        }
      })));
      return _el$84;
    })()), _el$80);
    insert(_el$81, createComponent(SidebarLinkWithBadge, {
      href: "/insights",
      icon: "📈",
      label: "Insights"
    }), null);
    insert(_el$81, createComponent(SidebarLink, {
      href: "/sync",
      icon: "⇄",
      label: "Sync"
    }), null);
    insert(_el$81, createComponent(SidebarLink, {
      href: "/audit",
      icon: "🛡",
      label: "Audit"
    }), null);
    insert(_el$81, createComponent(SidebarLink, {
      href: "/settings",
      icon: "⚙",
      label: "Settings"
    }), null);
    _el$83.$$click = lockNow;
    createRenderEffect(() => setAttribute(_el$77, "title", state.status().username ?? ""));
    return _el$72;
  })();
}
function SidebarLink(props) {
  return createComponent(A$1, {
    get href() {
      return props.href;
    },
    get end() {
      return props.end;
    },
    activeClass: "active",
    get children() {
      return [(() => {
        var _el$87 = _tmpl$1$2();
        insert(_el$87, () => props.icon);
        return _el$87;
      })(), (() => {
        var _el$88 = _tmpl$10$2();
        insert(_el$88, () => props.label);
        return _el$88;
      })()];
    }
  });
}
function SidebarLinkWithBadge(props) {
  const [insights] = createResource(async () => {
    try {
      return await generateInsights();
    } catch {
      return [];
    }
  });
  const warns = () => (insights() ?? []).filter((i) => i.severity === "warn").length;
  return createComponent(A$1, {
    get href() {
      return props.href;
    },
    activeClass: "active",
    get children() {
      return [(() => {
        var _el$89 = _tmpl$1$2();
        insert(_el$89, () => props.icon);
        return _el$89;
      })(), (() => {
        var _el$90 = _tmpl$10$2();
        insert(_el$90, () => props.label);
        return _el$90;
      })(), createComponent(Show, {
        get when() {
          return warns() > 0;
        },
        get children() {
          var _el$91 = _tmpl$11$1();
          insert(_el$91, warns);
          return _el$91;
        }
      })];
    }
  });
}
delegateEvents(["input", "click"]);
var _tmpl$$8 = /* @__PURE__ */ template(`<section class=dashboard-section><header class=dashboard-section-header><h2>Worth looking at</h2></header><div class=insights-list>`), _tmpl$2$7 = /* @__PURE__ */ template(`<section class=dashboard-section><header class=dashboard-section-header><h2>Recent activity</h2></header><div class=dashboard-activity>`), _tmpl$3$7 = /* @__PURE__ */ template(`<div class=page><div class=dashboard-hero><div><h1>.</h1><p>Your encrypted vault.</p></div><div class=dashboard-hero-stat><span class=dashboard-hero-stat-value></span><span class=dashboard-hero-stat-label>records</span></div></div><section class=dashboard-section><header class=dashboard-section-header><h2>Categories</h2></header><div class=dashboard-groups>`), _tmpl$4$7 = /* @__PURE__ */ template(`<div><div class=insight-icon></div><div class=insight-body><div class=insight-title></div><div class=insight-detail>`), _tmpl$5$7 = /* @__PURE__ */ template(`<div class=dashboard-group-info><h3></h3><div class=dashboard-group-types>`), _tmpl$6$5 = /* @__PURE__ */ template(`<div class=dashboard-group-count>`), _tmpl$7$5 = /* @__PURE__ */ template(`<span class=dashboard-group-type><span class=ic>`), _tmpl$8$4 = /* @__PURE__ */ template(`<div class=dashboard-activity-row><span class=dashboard-activity-time></span><span class=dashboard-activity-op></span><span class=dashboard-activity-actor>`);
function Dashboard() {
  const [counts, {
    refetch: refetchCounts
  }] = createResource(async () => {
    const out = {};
    for (const t of RECORD_TYPES) {
      try {
        const list2 = await api.listRecords(t.type);
        out[t.type] = list2.length;
      } catch {
        out[t.type] = 0;
      }
    }
    return out;
  });
  const [activity] = createResource(async () => {
    try {
      const audit = await api.audit(false);
      return audit.slice(0, 6);
    } catch {
      return [];
    }
  });
  const [insights] = createResource(async () => {
    try {
      return await generateInsights();
    } catch {
      return [];
    }
  });
  const topInsights = () => (insights() ?? []).slice(0, 2);
  const greeting = () => {
    const h = (/* @__PURE__ */ new Date()).getHours();
    if (h < 5) return "Working late";
    if (h < 12) return "Good morning";
    if (h < 17) return "Good afternoon";
    if (h < 21) return "Good evening";
    return "Working late";
  };
  onMount(() => {
  });
  const groups = () => {
    const c = counts();
    if (!c) return [];
    const map = /* @__PURE__ */ new Map();
    for (const t of RECORD_TYPES) {
      let g2 = map.get(t.group);
      if (!g2) {
        g2 = {
          name: t.group,
          total: 0,
          types: []
        };
        map.set(t.group, g2);
      }
      g2.types.push(t);
      g2.total += c[t.type] ?? 0;
    }
    return Array.from(map.values());
  };
  const totalRecords = () => {
    const c = counts();
    if (!c) return 0;
    return Object.values(c).reduce((s, n) => s + n, 0);
  };
  return (() => {
    var _el$ = _tmpl$3$7(), _el$2 = _el$.firstChild, _el$3 = _el$2.firstChild, _el$4 = _el$3.firstChild, _el$5 = _el$4.firstChild, _el$6 = _el$3.nextSibling, _el$7 = _el$6.firstChild, _el$10 = _el$2.nextSibling, _el$11 = _el$10.firstChild, _el$12 = _el$11.nextSibling;
    insert(_el$4, greeting, _el$5);
    insert(_el$7, totalRecords);
    insert(_el$, createComponent(Show, {
      get when() {
        return topInsights().length > 0;
      },
      get children() {
        var _el$8 = _tmpl$$8(), _el$9 = _el$8.firstChild;
        _el$9.firstChild;
        var _el$1 = _el$9.nextSibling;
        insert(_el$9, createComponent(A$1, {
          "class": "muted-small",
          href: "/insights",
          children: "View all →"
        }), null);
        insert(_el$1, createComponent(For, {
          get each() {
            return topInsights();
          },
          children: (i) => (() => {
            var _el$17 = _tmpl$4$7(), _el$18 = _el$17.firstChild, _el$19 = _el$18.nextSibling, _el$20 = _el$19.firstChild, _el$21 = _el$20.nextSibling;
            insert(_el$18, (() => {
              var _c$ = memo(() => i.severity === "warn");
              return () => _c$() ? "⚠" : i.severity === "ok" ? "✓" : "ⓘ";
            })());
            insert(_el$20, () => i.title);
            insert(_el$21, () => i.detail);
            insert(_el$17, createComponent(Show, {
              get when() {
                return i.to;
              },
              get children() {
                return createComponent(A$1, {
                  "class": "insight-action",
                  get href() {
                    return i.to;
                  },
                  children: "Open →"
                });
              }
            }), null);
            createRenderEffect(() => className(_el$17, `insight insight-${i.severity}`));
            return _el$17;
          })()
        }));
        return _el$8;
      }
    }), _el$10);
    insert(_el$12, createComponent(For, {
      get each() {
        return groups();
      },
      children: (g2) => createComponent(A$1, {
        "class": "dashboard-group",
        get href() {
          return firstCategoryHref(g2.name);
        },
        get children() {
          return [(() => {
            var _el$22 = _tmpl$5$7(), _el$23 = _el$22.firstChild, _el$24 = _el$23.nextSibling;
            insert(_el$23, () => g2.name);
            insert(_el$24, createComponent(For, {
              get each() {
                return g2.types;
              },
              children: (t) => (() => {
                var _el$26 = _tmpl$7$5(), _el$27 = _el$26.firstChild;
                insert(_el$27, () => t.icon);
                insert(_el$26, () => t.label, null);
                return _el$26;
              })()
            }));
            return _el$22;
          })(), (() => {
            var _el$25 = _tmpl$6$5();
            insert(_el$25, () => g2.total);
            return _el$25;
          })()];
        }
      })
    }));
    insert(_el$, createComponent(Show, {
      get when() {
        return memo(() => !!activity())() && activity().length > 0;
      },
      get children() {
        var _el$13 = _tmpl$2$7(), _el$14 = _el$13.firstChild;
        _el$14.firstChild;
        var _el$16 = _el$14.nextSibling;
        insert(_el$14, createComponent(A$1, {
          "class": "muted-small",
          href: "/audit",
          children: "View all →"
        }), null);
        insert(_el$16, createComponent(For, {
          get each() {
            return activity();
          },
          children: (a) => (() => {
            var _el$28 = _tmpl$8$4(), _el$29 = _el$28.firstChild, _el$30 = _el$29.nextSibling, _el$31 = _el$30.nextSibling;
            insert(_el$29, () => new Date(a.ts).toLocaleString());
            insert(_el$30, () => a.op);
            insert(_el$31, () => a.actor);
            return _el$28;
          })()
        }));
        return _el$13;
      }
    }), null);
    return _el$;
  })();
}
function firstCategoryHref(groupName) {
  const t = RECORD_TYPES.find((r) => r.group === groupName);
  return t ? `/c/${t.type}` : "/";
}
function joinList(record, field) {
  const v2 = record[field];
  if (Array.isArray(v2)) return v2.join(", ");
  if (v2 == null) return "";
  return String(v2);
}
function stripScheme(url) {
  if (!url) return "";
  return url.replace(/^https?:\/\//, "").replace(/\/$/, "");
}
function preview(body) {
  if (!body) return "";
  const oneLine = body.replace(/\s+/g, " ").trim();
  return oneLine.length > 80 ? oneLine.slice(0, 80) + "…" : oneLine;
}
const COLUMNS_BY_TYPE = {
  login: [
    { label: "Service", flex: 2, field: "service" },
    { label: "Username", flex: 2, field: "username" },
    { label: "Holders", flex: 2, format: (r) => joinList(r, "holders") },
    { label: "URL", flex: 2, format: (r) => stripScheme(r.url) }
  ],
  document: [
    { label: "Title", flex: 3, field: "title" },
    { label: "Type", flex: 2, field: "document_type" },
    { label: "Owner", flex: 1, field: "owner" },
    { label: "Expires", flex: 1, field: "expires_on" }
  ],
  identification: [
    { label: "Holder", flex: 2, field: "holder" },
    { label: "Type", flex: 2, field: "id_type" },
    { label: "Issuer", flex: 2, field: "issuer" },
    { label: "Expires", flex: 1, field: "expires_on" }
  ],
  insurance: [
    { label: "Type", flex: 2, field: "policy_type" },
    { label: "Provider", flex: 2, field: "provider" },
    { label: "Insured", flex: 2, format: (r) => joinList(r, "holders") },
    { label: "Renewal", flex: 1, field: "renewal_on" }
  ],
  health: [
    { label: "Subject", flex: 2, field: "subject" },
    { label: "Title", flex: 3, field: "title" }
  ],
  bank_account: [
    { label: "Bank", flex: 3, field: "bank" },
    { label: "Type", flex: 2, field: "account_type" },
    { label: "Holders", flex: 3, format: (r) => joinList(r, "holders") }
  ],
  credit_card: [
    { label: "Issuer", flex: 2, field: "issuer" },
    { label: "Network", flex: 1, field: "network" },
    { label: "Cardholders", flex: 3, format: (r) => joinList(r, "holders") },
    { label: "Expires", flex: 1, field: "expiration" }
  ],
  investment: [
    { label: "Provider", flex: 2, field: "provider" },
    { label: "Type", flex: 2, field: "account_type" },
    { label: "Holders", flex: 2, format: (r) => joinList(r, "holders") }
  ],
  income_source: [
    { label: "Source", flex: 3, field: "source" },
    { label: "Type", flex: 2, field: "income_type" },
    { label: "Schedule", flex: 2, field: "schedule" },
    { label: "Rate", flex: 1, field: "rate" }
  ],
  vehicle: [
    { label: "Year", flex: 1, field: "year" },
    { label: "Make/Model", flex: 3, field: "make_model" },
    { label: "Drivers", flex: 3, format: (r) => joinList(r, "drivers") },
    { label: "Plate", flex: 1, field: "license_plate" }
  ],
  residence: [
    { label: "Address", flex: 4, field: "address" },
    { label: "Leaseholders", flex: 2, format: (r) => joinList(r, "leaseholders") },
    { label: "Type", flex: 1, field: "residence_type" },
    { label: "Rent", flex: 1, field: "rent" }
  ],
  phone: [
    { label: "Device", flex: 2, field: "device" },
    { label: "Number", flex: 2, field: "phone_number" },
    { label: "Carrier", flex: 2, field: "carrier" },
    { label: "Users", flex: 2, format: (r) => joinList(r, "users") }
  ],
  address: [
    { label: "Label", flex: 2, field: "label" },
    { label: "Street", flex: 3, field: "street" },
    { label: "City", flex: 2, field: "city" }
  ],
  contact: [
    { label: "Name", flex: 3, field: "name" },
    { label: "Relationship", flex: 2, field: "relationship" },
    { label: "Email", flex: 2, field: "email" },
    { label: "Phone", flex: 2, field: "phone" }
  ],
  subscription: [
    { label: "Service", flex: 3, field: "service" },
    { label: "Cost", flex: 1, field: "cost" },
    { label: "Cycle", flex: 1, field: "cycle" },
    { label: "Holders", flex: 1, format: (r) => joinList(r, "holders") }
  ],
  infrastructure: [
    { label: "Name", flex: 2, field: "name" },
    { label: "Provider", flex: 2, field: "provider" },
    { label: "Type", flex: 2, field: "asset_type" },
    { label: "Holders", flex: 1, format: (r) => joinList(r, "holders") }
  ],
  domain: [
    { label: "FQDN", flex: 3, field: "fqdn" },
    { label: "Points to", flex: 2, field: "points_to" },
    { label: "Holders", flex: 1, format: (r) => joinList(r, "holders") }
  ],
  runbook: [
    { label: "Title", flex: 3, field: "title" },
    { label: "Description", flex: 4, field: "description" }
  ],
  work_log: [
    { label: "Date", flex: 1, field: "date" },
    { label: "Project", flex: 2, field: "project" },
    { label: "Summary", flex: 3, field: "summary" }
  ],
  note: [
    { label: "Title", flex: 3, field: "title" },
    { label: "Preview", flex: 4, format: (r) => preview(r.body) }
  ]
};
function renderCell(def, fields) {
  let raw;
  if (def.format) {
    raw = def.format(fields);
  } else if (def.field) {
    raw = fields[def.field];
  } else {
    raw = "";
  }
  if (raw == null) return "";
  if (Array.isArray(raw)) return raw.join(", ");
  if (typeof raw === "string") return raw;
  if (typeof raw === "number") return String(raw);
  return String(raw);
}
var _tmpl$$7 = /* @__PURE__ */ template(`<span class=muted> · `), _tmpl$2$6 = /* @__PURE__ */ template(`<div class=table-wrap><table class=rows><colgroup><col style=width:1px></colgroup><thead><tr><th class=col-actions style=text-align:right>actions</th></tr></thead><tbody>`), _tmpl$3$6 = /* @__PURE__ */ template(`<div class=page><header class=page-header><div><h1 class=page-title><span class=title-emoji></span></h1><p class=page-sub> record</p></div><div class=page-actions><input class=input placeholder=Filter… style=width:220px>`), _tmpl$4$6 = /* @__PURE__ */ template(`<p class=muted>Loading…`), _tmpl$5$6 = /* @__PURE__ */ template(`<div class=table-wrap><div class=empty-state><div class=empty-state-emoji></div><p class=empty-state-title>No <!> yet</p><p class=empty-state-sub>Add your first one to get started.`), _tmpl$6$4 = /* @__PURE__ */ template(`<col>`), _tmpl$7$4 = /* @__PURE__ */ template(`<th>`), _tmpl$8$3 = /* @__PURE__ */ template(`<tr><td class=actions><button type=button class=action-danger>delete`), _tmpl$9$2 = /* @__PURE__ */ template(`<td class=col-cell>`), _tmpl$0$2 = /* @__PURE__ */ template(`<span class=muted>—`);
function Category() {
  const params = useParams();
  const recordType = () => params.type;
  const meta = () => META_BY_TYPE[recordType()];
  const [filter, setFilter] = createSignal("");
  const [entries2, {
    refetch
  }] = createResource(() => params.type, async (t) => api.listRecords(t));
  async function del(id) {
    if (!confirm("Delete this record? This cannot be undone.")) return;
    try {
      await api.deleteRecord(id);
      showToast("ok", "Record deleted");
      refetch();
    } catch (e) {
      showToast("err", String(e));
    }
  }
  const [rows, {
    refetch: refetchRows
  }] = createResource(() => entries2(), async (list2) => {
    if (!list2) return [];
    const out = [];
    for (const e of list2) {
      try {
        const rec = await api.showRecord(e.id, false);
        out.push({
          id: e.id,
          fields: rec,
          updated_by: e.updated_by,
          updated_at: e.updated_at
        });
      } catch {
      }
    }
    return out;
  });
  const filtered = () => {
    const q2 = filter().toLowerCase().trim();
    const all = rows() ?? [];
    if (!q2) return all;
    return all.filter((r) => {
      const hay = Object.values(r.fields).join(" ").toLowerCase();
      return hay.includes(q2);
    });
  };
  const columns = () => COLUMNS_BY_TYPE[recordType()] ?? [];
  const totalFlex = () => {
    let total = 0;
    for (const c of columns()) {
      if (typeof c.flex === "number") total += c.flex;
    }
    return total || 1;
  };
  return (() => {
    var _el$ = _tmpl$3$6(), _el$2 = _el$.firstChild, _el$3 = _el$2.firstChild, _el$4 = _el$3.firstChild, _el$5 = _el$4.firstChild, _el$6 = _el$4.nextSibling, _el$7 = _el$6.firstChild, _el$0 = _el$3.nextSibling, _el$1 = _el$0.firstChild;
    insert(_el$5, () => meta()?.icon);
    insert(_el$4, () => meta()?.label ?? recordType(), null);
    insert(_el$6, () => entries2()?.length ?? 0, _el$7);
    insert(_el$6, () => (entries2()?.length ?? 0) === 1 ? "" : "s", null);
    insert(_el$6, createComponent(Show, {
      get when() {
        return meta()?.blurb;
      },
      get children() {
        var _el$8 = _tmpl$$7();
        _el$8.firstChild;
        insert(_el$8, () => meta().blurb, null);
        return _el$8;
      }
    }), null);
    _el$1.$$input = (e) => setFilter(e.currentTarget.value);
    insert(_el$0, createComponent(A$1, {
      "class": "btn btn-primary",
      get href() {
        return `/c/${params.type}/new`;
      },
      children: "+ New"
    }), null);
    insert(_el$, createComponent(Show, {
      get when() {
        return entries2() !== void 0;
      },
      get fallback() {
        return _tmpl$4$6();
      },
      get children() {
        return createComponent(Show, {
          get when() {
            return filtered().length > 0;
          },
          get fallback() {
            return (() => {
              var _el$19 = _tmpl$5$6(), _el$20 = _el$19.firstChild, _el$21 = _el$20.firstChild, _el$22 = _el$21.nextSibling, _el$23 = _el$22.firstChild, _el$25 = _el$23.nextSibling;
              _el$25.nextSibling;
              _el$22.nextSibling;
              insert(_el$21, () => meta()?.icon);
              insert(_el$22, () => meta()?.label.toLowerCase() ?? "records", _el$25);
              insert(_el$20, createComponent(A$1, {
                "class": "btn btn-primary",
                get href() {
                  return `/c/${params.type}/new`;
                },
                get children() {
                  return ["+ New ", memo(() => meta()?.label ?? "record")];
                }
              }), null);
              return _el$19;
            })();
          },
          get children() {
            var _el$10 = _tmpl$2$6(), _el$11 = _el$10.firstChild, _el$12 = _el$11.firstChild, _el$13 = _el$12.firstChild, _el$14 = _el$12.nextSibling, _el$15 = _el$14.firstChild, _el$16 = _el$15.firstChild, _el$17 = _el$14.nextSibling;
            insert(_el$12, createComponent(For, {
              get each() {
                return columns();
              },
              children: (c) => {
                const w2 = typeof c.flex === "number" ? `${c.flex / totalFlex() * 100}%` : c.flex;
                return (() => {
                  var _el$27 = _tmpl$6$4();
                  setStyleProperty(_el$27, "width", w2);
                  return _el$27;
                })();
              }
            }), _el$13);
            insert(_el$15, createComponent(For, {
              get each() {
                return columns();
              },
              children: (c) => (() => {
                var _el$28 = _tmpl$7$4();
                insert(_el$28, () => c.label);
                createRenderEffect(() => className(_el$28, c.align === "right" ? "col-right" : "col-left"));
                return _el$28;
              })()
            }), _el$16);
            insert(_el$17, createComponent(For, {
              get each() {
                return filtered();
              },
              children: (r) => (() => {
                var _el$29 = _tmpl$8$3(), _el$30 = _el$29.firstChild, _el$31 = _el$30.firstChild;
                insert(_el$29, createComponent(For, {
                  get each() {
                    return columns();
                  },
                  children: (c) => (() => {
                    var _el$32 = _tmpl$9$2();
                    insert(_el$32, createComponent(A$1, {
                      get href() {
                        return `/r/${r.id}`;
                      },
                      "class": "row-link",
                      get children() {
                        return renderCell(c, r.fields) || _tmpl$0$2();
                      }
                    }));
                    return _el$32;
                  })()
                }), _el$30);
                insert(_el$30, createComponent(A$1, {
                  get href() {
                    return `/r/${r.id}`;
                  },
                  children: "view"
                }), _el$31);
                insert(_el$30, createComponent(A$1, {
                  get href() {
                    return `/r/${r.id}/edit`;
                  },
                  children: "edit"
                }), _el$31);
                _el$31.$$click = () => del(r.id);
                return _el$29;
              })()
            }));
            return _el$10;
          }
        });
      }
    }), null);
    createRenderEffect(() => _el$1.value = filter());
    return _el$;
  })();
}
delegateEvents(["input", "click"]);
const SCHEMAS = {
  login: [
    { name: "service", label: "Service", required: true },
    { name: "username", label: "Username", required: true },
    { name: "holders", label: "Holders" },
    { name: "password", label: "Password", kind: "password" },
    { name: "totp_secret", label: "TOTP secret", kind: "password" },
    { name: "recovery_codes", label: "Recovery codes", multiline: true },
    { name: "url", label: "URL", kind: "url" },
    { name: "notes", label: "Notes", multiline: true }
  ],
  document: [
    { name: "title", label: "Title", required: true },
    { name: "document_type", label: "Type", required: true },
    { name: "owner", label: "Owner" },
    { name: "number", label: "Document #" },
    { name: "issuer", label: "Issuer" },
    { name: "issued_on", label: "Issued", kind: "date" },
    { name: "expires_on", label: "Expires", kind: "date" },
    { name: "location", label: "Location" },
    { name: "notes", label: "Notes", multiline: true }
  ],
  identification: [
    { name: "holder", label: "Holder", required: true },
    { name: "id_type", label: "ID type", required: true },
    { name: "issuer", label: "Issuer" },
    { name: "number", label: "Number", kind: "password", required: true },
    { name: "country", label: "Country" },
    { name: "class", label: "Class" },
    { name: "issued_on", label: "Issued", kind: "date" },
    { name: "expires_on", label: "Expires", kind: "date" },
    { name: "notes", label: "Notes", multiline: true }
  ],
  insurance: [
    { name: "policy_type", label: "Type", required: true },
    { name: "provider", label: "Provider", required: true },
    { name: "policy_number", label: "Policy #", required: true },
    { name: "group_number", label: "Group #" },
    { name: "member_id", label: "Member ID" },
    { name: "holders", label: "Insured" },
    { name: "beneficiary", label: "Beneficiary" },
    { name: "insured_item", label: "Insured item" },
    { name: "coverage", label: "Coverage" },
    { name: "deductible", label: "Deductible" },
    { name: "premium", label: "Premium" },
    { name: "effective_on", label: "Effective", kind: "date" },
    { name: "renewal_on", label: "Renewal", kind: "date" },
    { name: "agent", label: "Agent" },
    { name: "claims_phone", label: "Claims phone" },
    { name: "notes", label: "Notes", multiline: true }
  ],
  health: [
    { name: "subject", label: "Subject", required: true },
    { name: "title", label: "Title", required: true },
    { name: "details", label: "Details", multiline: true }
  ],
  bank_account: [
    { name: "bank", label: "Bank", required: true },
    { name: "account_type", label: "Type", required: true },
    { name: "holders", label: "Holders" },
    { name: "account_number", label: "Account #", kind: "password" },
    { name: "routing_number", label: "Routing #", kind: "password" },
    { name: "swift", label: "SWIFT / BIC" },
    { name: "branch", label: "Branch" },
    { name: "online_username", label: "Online username" },
    { name: "online_url", label: "Online URL", kind: "url" },
    { name: "notes", label: "Notes", multiline: true }
  ],
  credit_card: [
    { name: "issuer", label: "Issuer", required: true },
    { name: "network", label: "Network", required: true },
    { name: "holders", label: "Cardholders" },
    { name: "card_number", label: "Card #", kind: "password" },
    { name: "expiration", label: "Expiration" },
    { name: "cvv", label: "CVV", kind: "password" },
    { name: "pin", label: "Card PIN", kind: "password" },
    { name: "billing_address", label: "Billing address", multiline: true },
    { name: "issuer_phone", label: "Issuer phone" },
    { name: "issuer_url", label: "Issuer URL", kind: "url" },
    { name: "notes", label: "Notes", multiline: true }
  ],
  investment: [
    { name: "provider", label: "Provider", required: true },
    { name: "account_type", label: "Type", required: true },
    { name: "holders", label: "Holders" },
    { name: "notes", label: "Notes", multiline: true }
  ],
  income_source: [
    { name: "source", label: "Source", required: true },
    { name: "income_type", label: "Type", required: true },
    { name: "rate", label: "Rate" },
    { name: "schedule", label: "Schedule" },
    { name: "per_payment", label: "Per payment" },
    { name: "notes", label: "Notes", multiline: true }
  ],
  vehicle: [
    { name: "year", label: "Year", kind: "number", required: true },
    { name: "make_model", label: "Make / model", required: true },
    { name: "nickname", label: "Nickname" },
    { name: "drivers", label: "Drivers" },
    { name: "title_holder", label: "Title holder" },
    { name: "vin", label: "VIN" },
    { name: "license_plate", label: "Plate" },
    { name: "notes", label: "Notes", multiline: true }
  ],
  residence: [
    { name: "address", label: "Address", required: true, multiline: true },
    { name: "residence_type", label: "Type" },
    { name: "landlord", label: "Landlord" },
    { name: "leaseholders", label: "Leaseholders" },
    { name: "occupants", label: "Occupants" },
    { name: "rent", label: "Rent" },
    { name: "deposit", label: "Deposit" },
    { name: "notes", label: "Notes", multiline: true }
  ],
  phone: [
    { name: "device", label: "Device", required: true },
    { name: "model", label: "Model", required: true },
    { name: "phone_number", label: "Phone #", required: true, kind: "tel" },
    { name: "carrier", label: "Carrier", required: true },
    { name: "plan", label: "Plan" },
    { name: "users", label: "Users" },
    { name: "account_number", label: "Account #" },
    { name: "pin", label: "PIN", kind: "password" },
    { name: "notes", label: "Notes", multiline: true }
  ],
  address: [
    { name: "label", label: "Label", required: true },
    { name: "street", label: "Street", required: true, multiline: true },
    { name: "city", label: "City" },
    { name: "region", label: "State / region" },
    { name: "postal_code", label: "Postal code" },
    { name: "country", label: "Country" },
    { name: "notes", label: "Notes", multiline: true }
  ],
  contact: [
    { name: "name", label: "Name", required: true },
    { name: "relationship", label: "Relationship" },
    { name: "email", label: "Email", kind: "email" },
    { name: "phone", label: "Phone", kind: "tel" },
    { name: "notes", label: "Notes", multiline: true }
  ],
  subscription: [
    { name: "service", label: "Service", required: true },
    { name: "cost", label: "Cost", required: true },
    { name: "cycle", label: "Cycle", required: true },
    { name: "holders", label: "Holders" },
    { name: "username", label: "Username" },
    { name: "notes", label: "Notes", multiline: true }
  ],
  infrastructure: [
    { name: "name", label: "Name", required: true },
    { name: "provider", label: "Provider", required: true },
    { name: "asset_type", label: "Type", required: true },
    { name: "holders", label: "Holders" },
    { name: "notes", label: "Notes", multiline: true }
  ],
  domain: [
    { name: "fqdn", label: "FQDN", required: true },
    { name: "points_to", label: "Points to" },
    { name: "holders", label: "Contacts" },
    { name: "notes", label: "Notes", multiline: true }
  ],
  runbook: [
    { name: "title", label: "Title", required: true },
    { name: "description", label: "Description", required: true, multiline: true },
    { name: "steps", label: "Steps (title | body | status, one per line)", multiline: true },
    { name: "notes", label: "Notes", multiline: true }
  ],
  work_log: [
    { name: "date", label: "Date", kind: "date", required: true },
    { name: "project", label: "Project", required: true },
    { name: "summary", label: "Summary", required: true },
    { name: "details", label: "Details", multiline: true },
    { name: "tags", label: "Tags" }
  ],
  note: [
    { name: "title", label: "Title", required: true },
    { name: "body", label: "Body (markdown)", multiline: true },
    { name: "tags", label: "Tags" }
  ]
};
const SENSITIVE_FIELDS = /* @__PURE__ */ new Set([
  "password",
  "totp_secret",
  "recovery_codes",
  "number",
  "account_number",
  "routing_number",
  "online_username",
  "card_number",
  "expiration",
  "cvv",
  "pin",
  "issuer_phone",
  "claims_phone"
]);
function isSensitive(fieldName, schema) {
  if (SENSITIVE_FIELDS.has(fieldName)) return true;
  if (schema?.kind === "password") return true;
  return false;
}
/*! @license DOMPurify 3.4.11 | (c) Cure53 and other contributors | Released under the Apache license 2.0 and Mozilla Public License 2.0 | github.com/cure53/DOMPurify/blob/3.4.11/LICENSE */
function _arrayLikeToArray(r, a) {
  (null == a || a > r.length) && (a = r.length);
  for (var e = 0, n = Array(a); e < a; e++) n[e] = r[e];
  return n;
}
function _arrayWithHoles(r) {
  if (Array.isArray(r)) return r;
}
function _iterableToArrayLimit(r, l3) {
  var t = null == r ? null : "undefined" != typeof Symbol && r[Symbol.iterator] || r["@@iterator"];
  if (null != t) {
    var e, n, i, u, a = [], f = true, o = false;
    try {
      if (i = (t = t.call(r)).next, 0 === l3) ;
      else for (; !(f = (e = i.call(t)).done) && (a.push(e.value), a.length !== l3); f = true) ;
    } catch (r2) {
      o = true, n = r2;
    } finally {
      try {
        if (!f && null != t.return && (u = t.return(), Object(u) !== u)) return;
      } finally {
        if (o) throw n;
      }
    }
    return a;
  }
}
function _nonIterableRest() {
  throw new TypeError("Invalid attempt to destructure non-iterable instance.\nIn order to be iterable, non-array objects must have a [Symbol.iterator]() method.");
}
function _slicedToArray(r, e) {
  return _arrayWithHoles(r) || _iterableToArrayLimit(r, e) || _unsupportedIterableToArray(r, e) || _nonIterableRest();
}
function _unsupportedIterableToArray(r, a) {
  if (r) {
    if ("string" == typeof r) return _arrayLikeToArray(r, a);
    var t = {}.toString.call(r).slice(8, -1);
    return "Object" === t && r.constructor && (t = r.constructor.name), "Map" === t || "Set" === t ? Array.from(r) : "Arguments" === t || /^(?:Ui|I)nt(?:8|16|32)(?:Clamped)?Array$/.test(t) ? _arrayLikeToArray(r, a) : void 0;
  }
}
const entries = Object.entries, setPrototypeOf = Object.setPrototypeOf, isFrozen = Object.isFrozen, getPrototypeOf = Object.getPrototypeOf, getOwnPropertyDescriptor = Object.getOwnPropertyDescriptor;
let freeze = Object.freeze, seal = Object.seal, create = Object.create;
let _ref = typeof Reflect !== "undefined" && Reflect, apply = _ref.apply, construct = _ref.construct;
if (!freeze) {
  freeze = function freeze2(x2) {
    return x2;
  };
}
if (!seal) {
  seal = function seal2(x2) {
    return x2;
  };
}
if (!apply) {
  apply = function apply2(func, thisArg) {
    for (var _len = arguments.length, args = new Array(_len > 2 ? _len - 2 : 0), _key = 2; _key < _len; _key++) {
      args[_key - 2] = arguments[_key];
    }
    return func.apply(thisArg, args);
  };
}
if (!construct) {
  construct = function construct2(Func) {
    for (var _len2 = arguments.length, args = new Array(_len2 > 1 ? _len2 - 1 : 0), _key2 = 1; _key2 < _len2; _key2++) {
      args[_key2 - 1] = arguments[_key2];
    }
    return new Func(...args);
  };
}
const arrayForEach = unapply(Array.prototype.forEach);
const arrayLastIndexOf = unapply(Array.prototype.lastIndexOf);
const arrayPop = unapply(Array.prototype.pop);
const arrayPush = unapply(Array.prototype.push);
const arraySplice = unapply(Array.prototype.splice);
const arrayIsArray = Array.isArray;
const stringToLowerCase = unapply(String.prototype.toLowerCase);
const stringToString = unapply(String.prototype.toString);
const stringMatch = unapply(String.prototype.match);
const stringReplace = unapply(String.prototype.replace);
const stringIndexOf = unapply(String.prototype.indexOf);
const stringTrim = unapply(String.prototype.trim);
const numberToString = unapply(Number.prototype.toString);
const booleanToString = unapply(Boolean.prototype.toString);
const bigintToString = typeof BigInt === "undefined" ? null : unapply(BigInt.prototype.toString);
const symbolToString = typeof Symbol === "undefined" ? null : unapply(Symbol.prototype.toString);
const objectHasOwnProperty = unapply(Object.prototype.hasOwnProperty);
const objectToString = unapply(Object.prototype.toString);
const regExpTest = unapply(RegExp.prototype.test);
const typeErrorCreate = unconstruct(TypeError);
function unapply(func) {
  return function(thisArg) {
    if (thisArg instanceof RegExp) {
      thisArg.lastIndex = 0;
    }
    for (var _len3 = arguments.length, args = new Array(_len3 > 1 ? _len3 - 1 : 0), _key3 = 1; _key3 < _len3; _key3++) {
      args[_key3 - 1] = arguments[_key3];
    }
    return apply(func, thisArg, args);
  };
}
function unconstruct(Func) {
  return function() {
    for (var _len4 = arguments.length, args = new Array(_len4), _key4 = 0; _key4 < _len4; _key4++) {
      args[_key4] = arguments[_key4];
    }
    return construct(Func, args);
  };
}
function addToSet(set, array) {
  let transformCaseFunc = arguments.length > 2 && arguments[2] !== void 0 ? arguments[2] : stringToLowerCase;
  if (setPrototypeOf) {
    setPrototypeOf(set, null);
  }
  if (!arrayIsArray(array)) {
    return set;
  }
  let l3 = array.length;
  while (l3--) {
    let element = array[l3];
    if (typeof element === "string") {
      const lcElement = transformCaseFunc(element);
      if (lcElement !== element) {
        if (!isFrozen(array)) {
          array[l3] = lcElement;
        }
        element = lcElement;
      }
    }
    set[element] = true;
  }
  return set;
}
function cleanArray(array) {
  for (let index = 0; index < array.length; index++) {
    const isPropertyExist = objectHasOwnProperty(array, index);
    if (!isPropertyExist) {
      array[index] = null;
    }
  }
  return array;
}
function clone(object) {
  const newObject = create(null);
  for (const _ref2 of entries(object)) {
    var _ref3 = _slicedToArray(_ref2, 2);
    const property = _ref3[0];
    const value = _ref3[1];
    const isPropertyExist = objectHasOwnProperty(object, property);
    if (isPropertyExist) {
      if (arrayIsArray(value)) {
        newObject[property] = cleanArray(value);
      } else if (value && typeof value === "object" && value.constructor === Object) {
        newObject[property] = clone(value);
      } else {
        newObject[property] = value;
      }
    }
  }
  return newObject;
}
function stringifyValue(value) {
  switch (typeof value) {
    case "string": {
      return value;
    }
    case "number": {
      return numberToString(value);
    }
    case "boolean": {
      return booleanToString(value);
    }
    case "bigint": {
      return bigintToString ? bigintToString(value) : "0";
    }
    case "symbol": {
      return symbolToString ? symbolToString(value) : "Symbol()";
    }
    case "undefined": {
      return objectToString(value);
    }
    case "function":
    case "object": {
      if (value === null) {
        return objectToString(value);
      }
      const valueAsRecord = value;
      const valueToString = lookupGetter(valueAsRecord, "toString");
      if (typeof valueToString === "function") {
        const stringified = valueToString(valueAsRecord);
        return typeof stringified === "string" ? stringified : objectToString(stringified);
      }
      return objectToString(value);
    }
    default: {
      return objectToString(value);
    }
  }
}
function lookupGetter(object, prop) {
  while (object !== null) {
    const desc = getOwnPropertyDescriptor(object, prop);
    if (desc) {
      if (desc.get) {
        return unapply(desc.get);
      }
      if (typeof desc.value === "function") {
        return unapply(desc.value);
      }
    }
    object = getPrototypeOf(object);
  }
  function fallbackValue() {
    return null;
  }
  return fallbackValue;
}
function isRegex(value) {
  try {
    regExpTest(value, "");
    return true;
  } catch (_unused) {
    return false;
  }
}
const html$1 = freeze(["a", "abbr", "acronym", "address", "area", "article", "aside", "audio", "b", "bdi", "bdo", "big", "blink", "blockquote", "body", "br", "button", "canvas", "caption", "center", "cite", "code", "col", "colgroup", "content", "data", "datalist", "dd", "decorator", "del", "details", "dfn", "dialog", "dir", "div", "dl", "dt", "element", "em", "fieldset", "figcaption", "figure", "font", "footer", "form", "h1", "h2", "h3", "h4", "h5", "h6", "head", "header", "hgroup", "hr", "html", "i", "img", "input", "ins", "kbd", "label", "legend", "li", "main", "map", "mark", "marquee", "menu", "menuitem", "meter", "nav", "nobr", "ol", "optgroup", "option", "output", "p", "picture", "pre", "progress", "q", "rp", "rt", "ruby", "s", "samp", "search", "section", "select", "shadow", "slot", "small", "source", "spacer", "span", "strike", "strong", "style", "sub", "summary", "sup", "table", "tbody", "td", "template", "textarea", "tfoot", "th", "thead", "time", "tr", "track", "tt", "u", "ul", "var", "video", "wbr"]);
const svg$1 = freeze(["svg", "a", "altglyph", "altglyphdef", "altglyphitem", "animatecolor", "animatemotion", "animatetransform", "circle", "clippath", "defs", "desc", "ellipse", "enterkeyhint", "exportparts", "filter", "font", "g", "glyph", "glyphref", "hkern", "image", "inputmode", "line", "lineargradient", "marker", "mask", "metadata", "mpath", "part", "path", "pattern", "polygon", "polyline", "radialgradient", "rect", "stop", "style", "switch", "symbol", "text", "textpath", "title", "tref", "tspan", "view", "vkern"]);
const svgFilters = freeze(["feBlend", "feColorMatrix", "feComponentTransfer", "feComposite", "feConvolveMatrix", "feDiffuseLighting", "feDisplacementMap", "feDistantLight", "feDropShadow", "feFlood", "feFuncA", "feFuncB", "feFuncG", "feFuncR", "feGaussianBlur", "feImage", "feMerge", "feMergeNode", "feMorphology", "feOffset", "fePointLight", "feSpecularLighting", "feSpotLight", "feTile", "feTurbulence"]);
const svgDisallowed = freeze(["animate", "color-profile", "cursor", "discard", "font-face", "font-face-format", "font-face-name", "font-face-src", "font-face-uri", "foreignobject", "hatch", "hatchpath", "mesh", "meshgradient", "meshpatch", "meshrow", "missing-glyph", "script", "set", "solidcolor", "unknown", "use"]);
const mathMl$1 = freeze(["math", "menclose", "merror", "mfenced", "mfrac", "mglyph", "mi", "mlabeledtr", "mmultiscripts", "mn", "mo", "mover", "mpadded", "mphantom", "mroot", "mrow", "ms", "mspace", "msqrt", "mstyle", "msub", "msup", "msubsup", "mtable", "mtd", "mtext", "mtr", "munder", "munderover", "mprescripts"]);
const mathMlDisallowed = freeze(["maction", "maligngroup", "malignmark", "mlongdiv", "mscarries", "mscarry", "msgroup", "mstack", "msline", "msrow", "semantics", "annotation", "annotation-xml", "mprescripts", "none"]);
const text = freeze(["#text"]);
const html = freeze(["accept", "action", "align", "alt", "autocapitalize", "autocomplete", "autopictureinpicture", "autoplay", "background", "bgcolor", "border", "capture", "cellpadding", "cellspacing", "checked", "cite", "class", "clear", "color", "cols", "colspan", "command", "commandfor", "controls", "controlslist", "coords", "crossorigin", "datetime", "decoding", "default", "dir", "disabled", "disablepictureinpicture", "disableremoteplayback", "download", "draggable", "enctype", "enterkeyhint", "exportparts", "face", "for", "headers", "height", "hidden", "high", "href", "hreflang", "id", "inert", "inputmode", "integrity", "ismap", "kind", "label", "lang", "list", "loading", "loop", "low", "max", "maxlength", "media", "method", "min", "minlength", "multiple", "muted", "name", "nonce", "noshade", "novalidate", "nowrap", "open", "optimum", "part", "pattern", "placeholder", "playsinline", "popover", "popovertarget", "popovertargetaction", "poster", "preload", "pubdate", "radiogroup", "readonly", "rel", "required", "rev", "reversed", "role", "rows", "rowspan", "spellcheck", "scope", "selected", "shape", "size", "sizes", "slot", "span", "srclang", "start", "src", "srcset", "step", "style", "summary", "tabindex", "title", "translate", "type", "usemap", "valign", "value", "width", "wrap", "xmlns"]);
const svg = freeze(["accent-height", "accumulate", "additive", "alignment-baseline", "amplitude", "ascent", "attributename", "attributetype", "azimuth", "basefrequency", "baseline-shift", "begin", "bias", "by", "class", "clip", "clippathunits", "clip-path", "clip-rule", "color", "color-interpolation", "color-interpolation-filters", "color-profile", "color-rendering", "cx", "cy", "d", "dx", "dy", "diffuseconstant", "direction", "display", "divisor", "dur", "edgemode", "elevation", "end", "exponent", "fill", "fill-opacity", "fill-rule", "filter", "filterunits", "flood-color", "flood-opacity", "font-family", "font-size", "font-size-adjust", "font-stretch", "font-style", "font-variant", "font-weight", "fx", "fy", "g1", "g2", "glyph-name", "glyphref", "gradientunits", "gradienttransform", "height", "href", "id", "image-rendering", "in", "in2", "intercept", "k", "k1", "k2", "k3", "k4", "kerning", "keypoints", "keysplines", "keytimes", "lang", "lengthadjust", "letter-spacing", "kernelmatrix", "kernelunitlength", "lighting-color", "local", "marker-end", "marker-mid", "marker-start", "markerheight", "markerunits", "markerwidth", "maskcontentunits", "maskunits", "max", "mask", "mask-type", "media", "method", "mode", "min", "name", "numoctaves", "offset", "operator", "opacity", "order", "orient", "orientation", "origin", "overflow", "paint-order", "path", "pathlength", "patterncontentunits", "patterntransform", "patternunits", "points", "preservealpha", "preserveaspectratio", "primitiveunits", "r", "rx", "ry", "radius", "refx", "refy", "repeatcount", "repeatdur", "restart", "result", "rotate", "scale", "seed", "shape-rendering", "slope", "specularconstant", "specularexponent", "spreadmethod", "startoffset", "stddeviation", "stitchtiles", "stop-color", "stop-opacity", "stroke-dasharray", "stroke-dashoffset", "stroke-linecap", "stroke-linejoin", "stroke-miterlimit", "stroke-opacity", "stroke", "stroke-width", "style", "surfacescale", "systemlanguage", "tabindex", "tablevalues", "targetx", "targety", "transform", "transform-origin", "text-anchor", "text-decoration", "text-rendering", "textlength", "type", "u1", "u2", "unicode", "values", "viewbox", "visibility", "version", "vert-adv-y", "vert-origin-x", "vert-origin-y", "width", "word-spacing", "wrap", "writing-mode", "xchannelselector", "ychannelselector", "x", "x1", "x2", "xmlns", "y", "y1", "y2", "z", "zoomandpan"]);
const mathMl = freeze(["accent", "accentunder", "align", "bevelled", "close", "columnalign", "columnlines", "columnspacing", "columnspan", "denomalign", "depth", "dir", "display", "displaystyle", "encoding", "fence", "frame", "height", "href", "id", "largeop", "length", "linethickness", "lquote", "lspace", "mathbackground", "mathcolor", "mathsize", "mathvariant", "maxsize", "minsize", "movablelimits", "notation", "numalign", "open", "rowalign", "rowlines", "rowspacing", "rowspan", "rspace", "rquote", "scriptlevel", "scriptminsize", "scriptsizemultiplier", "selection", "separator", "separators", "stretchy", "subscriptshift", "supscriptshift", "symmetric", "voffset", "width", "xmlns"]);
const xml = freeze(["xlink:href", "xml:id", "xlink:title", "xml:space", "xmlns:xlink"]);
const MUSTACHE_EXPR = seal(/{{[\w\W]*|^[\w\W]*}}/g);
const ERB_EXPR = seal(/<%[\w\W]*|^[\w\W]*%>/g);
const TMPLIT_EXPR = seal(/\${[\w\W]*/g);
const DATA_ATTR = seal(/^data-[\-\w.\u00B7-\uFFFF]+$/);
const ARIA_ATTR = seal(/^aria-[\-\w]+$/);
const IS_ALLOWED_URI = seal(
  /^(?:(?:(?:f|ht)tps?|mailto|tel|callto|sms|cid|xmpp|matrix):|[^a-z]|[a-z+.\-]+(?:[^a-z+.\-:]|$))/i
  // eslint-disable-line no-useless-escape
);
const IS_SCRIPT_OR_DATA = seal(/^(?:\w+script|data):/i);
const ATTR_WHITESPACE = seal(
  /[\u0000-\u0020\u00A0\u1680\u180E\u2000-\u2029\u205F\u3000]/g
  // eslint-disable-line no-control-regex
);
const DOCTYPE_NAME = seal(/^html$/i);
const CUSTOM_ELEMENT = seal(/^[a-z][.\w]*(-[.\w]+)+$/i);
const ELEMENT_MARKUP_PROBE = seal(/<[/\w!]/g);
const COMMENT_MARKUP_PROBE = seal(/<[/\w]/g);
const FALLBACK_TAG_CLOSE = seal(/<\/no(script|embed|frames)/i);
const SELF_CLOSING_TAG = seal(/\/>/i);
const NODE_TYPE = {
  element: 1,
  attribute: 2,
  text: 3,
  cdataSection: 4,
  entityReference: 5,
  // Deprecated
  entityNode: 6,
  // Deprecated
  processingInstruction: 7,
  comment: 8,
  document: 9,
  documentType: 10,
  documentFragment: 11,
  notation: 12
  // Deprecated
};
const getGlobal = function getGlobal2() {
  return typeof window === "undefined" ? null : window;
};
const _createTrustedTypesPolicy = function _createTrustedTypesPolicy2(trustedTypes, purifyHostElement) {
  if (typeof trustedTypes !== "object" || typeof trustedTypes.createPolicy !== "function") {
    return null;
  }
  let suffix = null;
  const ATTR_NAME = "data-tt-policy-suffix";
  if (purifyHostElement && purifyHostElement.hasAttribute(ATTR_NAME)) {
    suffix = purifyHostElement.getAttribute(ATTR_NAME);
  }
  const policyName = "dompurify" + (suffix ? "#" + suffix : "");
  try {
    return trustedTypes.createPolicy(policyName, {
      createHTML(html2) {
        return html2;
      },
      createScriptURL(scriptUrl) {
        return scriptUrl;
      }
    });
  } catch (_2) {
    console.warn("TrustedTypes policy " + policyName + " could not be created.");
    return null;
  }
};
const _createHooksMap = function _createHooksMap2() {
  return {
    afterSanitizeAttributes: [],
    afterSanitizeElements: [],
    afterSanitizeShadowDOM: [],
    beforeSanitizeAttributes: [],
    beforeSanitizeElements: [],
    beforeSanitizeShadowDOM: [],
    uponSanitizeAttribute: [],
    uponSanitizeElement: [],
    uponSanitizeShadowNode: []
  };
};
const _resolveSetOption = function _resolveSetOption2(cfg, key, fallback, options) {
  return objectHasOwnProperty(cfg, key) && arrayIsArray(cfg[key]) ? addToSet(options.base ? clone(options.base) : {}, cfg[key], options.transform) : fallback;
};
function createDOMPurify() {
  let window2 = arguments.length > 0 && arguments[0] !== void 0 ? arguments[0] : getGlobal();
  const DOMPurify = (root2) => createDOMPurify(root2);
  DOMPurify.version = "3.4.11";
  DOMPurify.removed = [];
  if (!window2 || !window2.document || window2.document.nodeType !== NODE_TYPE.document || !window2.Element) {
    DOMPurify.isSupported = false;
    return DOMPurify;
  }
  let document2 = window2.document;
  const originalDocument = document2;
  const currentScript = originalDocument.currentScript;
  window2.DocumentFragment;
  const HTMLTemplateElement = window2.HTMLTemplateElement, Node2 = window2.Node, Element2 = window2.Element, NodeFilter = window2.NodeFilter, _window$NamedNodeMap = window2.NamedNodeMap;
  _window$NamedNodeMap === void 0 ? window2.NamedNodeMap || window2.MozNamedAttrMap : _window$NamedNodeMap;
  window2.HTMLFormElement;
  const DOMParser = window2.DOMParser, trustedTypes = window2.trustedTypes;
  const ElementPrototype = Element2.prototype;
  const cloneNode = lookupGetter(ElementPrototype, "cloneNode");
  const remove = lookupGetter(ElementPrototype, "remove");
  const getNextSibling = lookupGetter(ElementPrototype, "nextSibling");
  const getChildNodes = lookupGetter(ElementPrototype, "childNodes");
  const getParentNode = lookupGetter(ElementPrototype, "parentNode");
  const getShadowRoot = lookupGetter(ElementPrototype, "shadowRoot");
  const getAttributes = lookupGetter(ElementPrototype, "attributes");
  const getNodeType = Node2 && Node2.prototype ? lookupGetter(Node2.prototype, "nodeType") : null;
  const getNodeName = Node2 && Node2.prototype ? lookupGetter(Node2.prototype, "nodeName") : null;
  if (typeof HTMLTemplateElement === "function") {
    const template2 = document2.createElement("template");
    if (template2.content && template2.content.ownerDocument) {
      document2 = template2.content.ownerDocument;
    }
  }
  let trustedTypesPolicy;
  let emptyHTML = "";
  let defaultTrustedTypesPolicy;
  let defaultTrustedTypesPolicyResolved = false;
  let IN_TRUSTED_TYPES_POLICY = 0;
  const _assertNotInTrustedTypesPolicy = function _assertNotInTrustedTypesPolicy2() {
    if (IN_TRUSTED_TYPES_POLICY > 0) {
      throw typeErrorCreate('A configured TRUSTED_TYPES_POLICY callback (createHTML or createScriptURL) must not call DOMPurify.sanitize, as that causes infinite recursion. Do not pass a policy whose callbacks wrap DOMPurify as TRUSTED_TYPES_POLICY; see the "DOMPurify and Trusted Types" section of the README.');
    }
  };
  const _createTrustedHTML = function _createTrustedHTML2(html2) {
    _assertNotInTrustedTypesPolicy();
    IN_TRUSTED_TYPES_POLICY++;
    try {
      return trustedTypesPolicy.createHTML(html2);
    } finally {
      IN_TRUSTED_TYPES_POLICY--;
    }
  };
  const _createTrustedScriptURL = function _createTrustedScriptURL2(scriptUrl) {
    _assertNotInTrustedTypesPolicy();
    IN_TRUSTED_TYPES_POLICY++;
    try {
      return trustedTypesPolicy.createScriptURL(scriptUrl);
    } finally {
      IN_TRUSTED_TYPES_POLICY--;
    }
  };
  const _getDefaultTrustedTypesPolicy = function _getDefaultTrustedTypesPolicy2() {
    if (!defaultTrustedTypesPolicyResolved) {
      defaultTrustedTypesPolicy = _createTrustedTypesPolicy(trustedTypes, currentScript);
      defaultTrustedTypesPolicyResolved = true;
    }
    return defaultTrustedTypesPolicy;
  };
  const _document = document2, implementation = _document.implementation, createNodeIterator = _document.createNodeIterator, createDocumentFragment = _document.createDocumentFragment, getElementsByTagName = _document.getElementsByTagName;
  const importNode = originalDocument.importNode;
  let hooks = _createHooksMap();
  DOMPurify.isSupported = typeof entries === "function" && typeof getParentNode === "function" && implementation && implementation.createHTMLDocument !== void 0;
  const MUSTACHE_EXPR$1 = MUSTACHE_EXPR, ERB_EXPR$1 = ERB_EXPR, TMPLIT_EXPR$1 = TMPLIT_EXPR, DATA_ATTR$1 = DATA_ATTR, ARIA_ATTR$1 = ARIA_ATTR, IS_SCRIPT_OR_DATA$1 = IS_SCRIPT_OR_DATA, ATTR_WHITESPACE$1 = ATTR_WHITESPACE, CUSTOM_ELEMENT$1 = CUSTOM_ELEMENT;
  let IS_ALLOWED_URI$1 = IS_ALLOWED_URI;
  let ALLOWED_TAGS = null;
  const DEFAULT_ALLOWED_TAGS = addToSet({}, [...html$1, ...svg$1, ...svgFilters, ...mathMl$1, ...text]);
  let ALLOWED_ATTR = null;
  const DEFAULT_ALLOWED_ATTR = addToSet({}, [...html, ...svg, ...mathMl, ...xml]);
  let CUSTOM_ELEMENT_HANDLING = Object.seal(create(null, {
    tagNameCheck: {
      writable: true,
      configurable: false,
      enumerable: true,
      value: null
    },
    attributeNameCheck: {
      writable: true,
      configurable: false,
      enumerable: true,
      value: null
    },
    allowCustomizedBuiltInElements: {
      writable: true,
      configurable: false,
      enumerable: true,
      value: false
    }
  }));
  let FORBID_TAGS = null;
  let FORBID_ATTR = null;
  const EXTRA_ELEMENT_HANDLING = Object.seal(create(null, {
    tagCheck: {
      writable: true,
      configurable: false,
      enumerable: true,
      value: null
    },
    attributeCheck: {
      writable: true,
      configurable: false,
      enumerable: true,
      value: null
    }
  }));
  let ALLOW_ARIA_ATTR = true;
  let ALLOW_DATA_ATTR = true;
  let ALLOW_UNKNOWN_PROTOCOLS = false;
  let ALLOW_SELF_CLOSE_IN_ATTR = true;
  let SAFE_FOR_TEMPLATES = false;
  let SAFE_FOR_XML = true;
  let WHOLE_DOCUMENT = false;
  let SET_CONFIG = false;
  let SET_CONFIG_ALLOWED_TAGS = null;
  let SET_CONFIG_ALLOWED_ATTR = null;
  let FORCE_BODY = false;
  let RETURN_DOM = false;
  let RETURN_DOM_FRAGMENT = false;
  let RETURN_TRUSTED_TYPE = false;
  let SANITIZE_DOM = true;
  let SANITIZE_NAMED_PROPS = false;
  const SANITIZE_NAMED_PROPS_PREFIX = "user-content-";
  let KEEP_CONTENT = true;
  let IN_PLACE = false;
  let USE_PROFILES = {};
  let FORBID_CONTENTS = null;
  const DEFAULT_FORBID_CONTENTS = addToSet({}, [
    "annotation-xml",
    "audio",
    "colgroup",
    "desc",
    "foreignobject",
    "head",
    "iframe",
    "math",
    "mi",
    "mn",
    "mo",
    "ms",
    "mtext",
    "noembed",
    "noframes",
    "noscript",
    "plaintext",
    "script",
    // <selectedcontent> mirrors the selected <option>'s subtree, cloned by
    // the UA (customizable <select>) — including any on* handlers — and the
    // engine re-mirrors synchronously whenever a removal changes which
    // option/selectedcontent is current, even inside DOMPurify's inert
    // DOMParser document. Hoisting its children on removal re-inserts a fresh
    // mirror target ahead of the walk, which the engine refills, looping
    // forever (DoS) and amplifying output. Dropping its content on removal
    // (rather than hoisting) breaks that cascade; the content is a duplicate
    // of the option, which is sanitized on its own. See campaign-3 F1/F6.
    "selectedcontent",
    "style",
    "svg",
    "template",
    "thead",
    "title",
    "video",
    "xmp"
  ]);
  let DATA_URI_TAGS = null;
  const DEFAULT_DATA_URI_TAGS = addToSet({}, ["audio", "video", "img", "source", "image", "track"]);
  let URI_SAFE_ATTRIBUTES = null;
  const DEFAULT_URI_SAFE_ATTRIBUTES = addToSet({}, ["alt", "class", "for", "id", "label", "name", "pattern", "placeholder", "role", "summary", "title", "value", "style", "xmlns"]);
  const MATHML_NAMESPACE = "http://www.w3.org/1998/Math/MathML";
  const SVG_NAMESPACE = "http://www.w3.org/2000/svg";
  const HTML_NAMESPACE = "http://www.w3.org/1999/xhtml";
  let NAMESPACE = HTML_NAMESPACE;
  let IS_EMPTY_INPUT = false;
  let ALLOWED_NAMESPACES = null;
  const DEFAULT_ALLOWED_NAMESPACES = addToSet({}, [MATHML_NAMESPACE, SVG_NAMESPACE, HTML_NAMESPACE], stringToString);
  const DEFAULT_MATHML_TEXT_INTEGRATION_POINTS = freeze(["mi", "mo", "mn", "ms", "mtext"]);
  let MATHML_TEXT_INTEGRATION_POINTS = addToSet({}, DEFAULT_MATHML_TEXT_INTEGRATION_POINTS);
  const DEFAULT_HTML_INTEGRATION_POINTS = freeze(["annotation-xml"]);
  let HTML_INTEGRATION_POINTS = addToSet({}, DEFAULT_HTML_INTEGRATION_POINTS);
  const COMMON_SVG_AND_HTML_ELEMENTS = addToSet({}, ["title", "style", "font", "a", "script"]);
  let PARSER_MEDIA_TYPE = null;
  const SUPPORTED_PARSER_MEDIA_TYPES = ["application/xhtml+xml", "text/html"];
  const DEFAULT_PARSER_MEDIA_TYPE = "text/html";
  let transformCaseFunc = null;
  let CONFIG = null;
  const formElement = document2.createElement("form");
  const isRegexOrFunction = function isRegexOrFunction2(testValue) {
    return testValue instanceof RegExp || testValue instanceof Function;
  };
  const _parseConfig = function _parseConfig2() {
    let cfg = arguments.length > 0 && arguments[0] !== void 0 ? arguments[0] : {};
    if (CONFIG && CONFIG === cfg) {
      return;
    }
    if (!cfg || typeof cfg !== "object") {
      cfg = {};
    }
    cfg = clone(cfg);
    PARSER_MEDIA_TYPE = // eslint-disable-next-line unicorn/prefer-includes
    SUPPORTED_PARSER_MEDIA_TYPES.indexOf(cfg.PARSER_MEDIA_TYPE) === -1 ? DEFAULT_PARSER_MEDIA_TYPE : cfg.PARSER_MEDIA_TYPE;
    transformCaseFunc = PARSER_MEDIA_TYPE === "application/xhtml+xml" ? stringToString : stringToLowerCase;
    ALLOWED_TAGS = _resolveSetOption(cfg, "ALLOWED_TAGS", DEFAULT_ALLOWED_TAGS, {
      transform: transformCaseFunc
    });
    ALLOWED_ATTR = _resolveSetOption(cfg, "ALLOWED_ATTR", DEFAULT_ALLOWED_ATTR, {
      transform: transformCaseFunc
    });
    ALLOWED_NAMESPACES = _resolveSetOption(cfg, "ALLOWED_NAMESPACES", DEFAULT_ALLOWED_NAMESPACES, {
      transform: stringToString
    });
    URI_SAFE_ATTRIBUTES = _resolveSetOption(cfg, "ADD_URI_SAFE_ATTR", DEFAULT_URI_SAFE_ATTRIBUTES, {
      transform: transformCaseFunc,
      base: DEFAULT_URI_SAFE_ATTRIBUTES
    });
    DATA_URI_TAGS = _resolveSetOption(cfg, "ADD_DATA_URI_TAGS", DEFAULT_DATA_URI_TAGS, {
      transform: transformCaseFunc,
      base: DEFAULT_DATA_URI_TAGS
    });
    FORBID_CONTENTS = _resolveSetOption(cfg, "FORBID_CONTENTS", DEFAULT_FORBID_CONTENTS, {
      transform: transformCaseFunc
    });
    FORBID_TAGS = _resolveSetOption(cfg, "FORBID_TAGS", clone({}), {
      transform: transformCaseFunc
    });
    FORBID_ATTR = _resolveSetOption(cfg, "FORBID_ATTR", clone({}), {
      transform: transformCaseFunc
    });
    USE_PROFILES = objectHasOwnProperty(cfg, "USE_PROFILES") ? cfg.USE_PROFILES && typeof cfg.USE_PROFILES === "object" ? clone(cfg.USE_PROFILES) : cfg.USE_PROFILES : false;
    ALLOW_ARIA_ATTR = cfg.ALLOW_ARIA_ATTR !== false;
    ALLOW_DATA_ATTR = cfg.ALLOW_DATA_ATTR !== false;
    ALLOW_UNKNOWN_PROTOCOLS = cfg.ALLOW_UNKNOWN_PROTOCOLS || false;
    ALLOW_SELF_CLOSE_IN_ATTR = cfg.ALLOW_SELF_CLOSE_IN_ATTR !== false;
    SAFE_FOR_TEMPLATES = cfg.SAFE_FOR_TEMPLATES || false;
    SAFE_FOR_XML = cfg.SAFE_FOR_XML !== false;
    WHOLE_DOCUMENT = cfg.WHOLE_DOCUMENT || false;
    RETURN_DOM = cfg.RETURN_DOM || false;
    RETURN_DOM_FRAGMENT = cfg.RETURN_DOM_FRAGMENT || false;
    RETURN_TRUSTED_TYPE = cfg.RETURN_TRUSTED_TYPE || false;
    FORCE_BODY = cfg.FORCE_BODY || false;
    SANITIZE_DOM = cfg.SANITIZE_DOM !== false;
    SANITIZE_NAMED_PROPS = cfg.SANITIZE_NAMED_PROPS || false;
    KEEP_CONTENT = cfg.KEEP_CONTENT !== false;
    IN_PLACE = cfg.IN_PLACE || false;
    IS_ALLOWED_URI$1 = isRegex(cfg.ALLOWED_URI_REGEXP) ? cfg.ALLOWED_URI_REGEXP : IS_ALLOWED_URI;
    NAMESPACE = typeof cfg.NAMESPACE === "string" ? cfg.NAMESPACE : HTML_NAMESPACE;
    MATHML_TEXT_INTEGRATION_POINTS = objectHasOwnProperty(cfg, "MATHML_TEXT_INTEGRATION_POINTS") && cfg.MATHML_TEXT_INTEGRATION_POINTS && typeof cfg.MATHML_TEXT_INTEGRATION_POINTS === "object" ? clone(cfg.MATHML_TEXT_INTEGRATION_POINTS) : addToSet({}, DEFAULT_MATHML_TEXT_INTEGRATION_POINTS);
    HTML_INTEGRATION_POINTS = objectHasOwnProperty(cfg, "HTML_INTEGRATION_POINTS") && cfg.HTML_INTEGRATION_POINTS && typeof cfg.HTML_INTEGRATION_POINTS === "object" ? clone(cfg.HTML_INTEGRATION_POINTS) : addToSet({}, DEFAULT_HTML_INTEGRATION_POINTS);
    const customElementHandling = objectHasOwnProperty(cfg, "CUSTOM_ELEMENT_HANDLING") && cfg.CUSTOM_ELEMENT_HANDLING && typeof cfg.CUSTOM_ELEMENT_HANDLING === "object" ? clone(cfg.CUSTOM_ELEMENT_HANDLING) : create(null);
    CUSTOM_ELEMENT_HANDLING = create(null);
    if (objectHasOwnProperty(customElementHandling, "tagNameCheck") && isRegexOrFunction(customElementHandling.tagNameCheck)) {
      CUSTOM_ELEMENT_HANDLING.tagNameCheck = customElementHandling.tagNameCheck;
    }
    if (objectHasOwnProperty(customElementHandling, "attributeNameCheck") && isRegexOrFunction(customElementHandling.attributeNameCheck)) {
      CUSTOM_ELEMENT_HANDLING.attributeNameCheck = customElementHandling.attributeNameCheck;
    }
    if (objectHasOwnProperty(customElementHandling, "allowCustomizedBuiltInElements") && typeof customElementHandling.allowCustomizedBuiltInElements === "boolean") {
      CUSTOM_ELEMENT_HANDLING.allowCustomizedBuiltInElements = customElementHandling.allowCustomizedBuiltInElements;
    }
    seal(CUSTOM_ELEMENT_HANDLING);
    if (SAFE_FOR_TEMPLATES) {
      ALLOW_DATA_ATTR = false;
    }
    if (RETURN_DOM_FRAGMENT) {
      RETURN_DOM = true;
    }
    if (USE_PROFILES) {
      ALLOWED_TAGS = addToSet({}, text);
      ALLOWED_ATTR = create(null);
      if (USE_PROFILES.html === true) {
        addToSet(ALLOWED_TAGS, html$1);
        addToSet(ALLOWED_ATTR, html);
      }
      if (USE_PROFILES.svg === true) {
        addToSet(ALLOWED_TAGS, svg$1);
        addToSet(ALLOWED_ATTR, svg);
        addToSet(ALLOWED_ATTR, xml);
      }
      if (USE_PROFILES.svgFilters === true) {
        addToSet(ALLOWED_TAGS, svgFilters);
        addToSet(ALLOWED_ATTR, svg);
        addToSet(ALLOWED_ATTR, xml);
      }
      if (USE_PROFILES.mathMl === true) {
        addToSet(ALLOWED_TAGS, mathMl$1);
        addToSet(ALLOWED_ATTR, mathMl);
        addToSet(ALLOWED_ATTR, xml);
      }
    }
    EXTRA_ELEMENT_HANDLING.tagCheck = null;
    EXTRA_ELEMENT_HANDLING.attributeCheck = null;
    if (objectHasOwnProperty(cfg, "ADD_TAGS")) {
      if (typeof cfg.ADD_TAGS === "function") {
        EXTRA_ELEMENT_HANDLING.tagCheck = cfg.ADD_TAGS;
      } else if (arrayIsArray(cfg.ADD_TAGS)) {
        if (ALLOWED_TAGS === DEFAULT_ALLOWED_TAGS) {
          ALLOWED_TAGS = clone(ALLOWED_TAGS);
        }
        addToSet(ALLOWED_TAGS, cfg.ADD_TAGS, transformCaseFunc);
      }
    }
    if (objectHasOwnProperty(cfg, "ADD_ATTR")) {
      if (typeof cfg.ADD_ATTR === "function") {
        EXTRA_ELEMENT_HANDLING.attributeCheck = cfg.ADD_ATTR;
      } else if (arrayIsArray(cfg.ADD_ATTR)) {
        if (ALLOWED_ATTR === DEFAULT_ALLOWED_ATTR) {
          ALLOWED_ATTR = clone(ALLOWED_ATTR);
        }
        addToSet(ALLOWED_ATTR, cfg.ADD_ATTR, transformCaseFunc);
      }
    }
    if (objectHasOwnProperty(cfg, "ADD_URI_SAFE_ATTR") && arrayIsArray(cfg.ADD_URI_SAFE_ATTR)) {
      addToSet(URI_SAFE_ATTRIBUTES, cfg.ADD_URI_SAFE_ATTR, transformCaseFunc);
    }
    if (objectHasOwnProperty(cfg, "FORBID_CONTENTS") && arrayIsArray(cfg.FORBID_CONTENTS)) {
      if (FORBID_CONTENTS === DEFAULT_FORBID_CONTENTS) {
        FORBID_CONTENTS = clone(FORBID_CONTENTS);
      }
      addToSet(FORBID_CONTENTS, cfg.FORBID_CONTENTS, transformCaseFunc);
    }
    if (objectHasOwnProperty(cfg, "ADD_FORBID_CONTENTS") && arrayIsArray(cfg.ADD_FORBID_CONTENTS)) {
      if (FORBID_CONTENTS === DEFAULT_FORBID_CONTENTS) {
        FORBID_CONTENTS = clone(FORBID_CONTENTS);
      }
      addToSet(FORBID_CONTENTS, cfg.ADD_FORBID_CONTENTS, transformCaseFunc);
    }
    if (KEEP_CONTENT) {
      ALLOWED_TAGS["#text"] = true;
    }
    if (WHOLE_DOCUMENT) {
      addToSet(ALLOWED_TAGS, ["html", "head", "body"]);
    }
    if (ALLOWED_TAGS.table) {
      addToSet(ALLOWED_TAGS, ["tbody"]);
      delete FORBID_TAGS.tbody;
    }
    if (cfg.TRUSTED_TYPES_POLICY) {
      if (typeof cfg.TRUSTED_TYPES_POLICY.createHTML !== "function") {
        throw typeErrorCreate('TRUSTED_TYPES_POLICY configuration option must provide a "createHTML" hook.');
      }
      if (typeof cfg.TRUSTED_TYPES_POLICY.createScriptURL !== "function") {
        throw typeErrorCreate('TRUSTED_TYPES_POLICY configuration option must provide a "createScriptURL" hook.');
      }
      const previousTrustedTypesPolicy = trustedTypesPolicy;
      trustedTypesPolicy = cfg.TRUSTED_TYPES_POLICY;
      try {
        emptyHTML = _createTrustedHTML("");
      } catch (error) {
        trustedTypesPolicy = previousTrustedTypesPolicy;
        throw error;
      }
    } else if (cfg.TRUSTED_TYPES_POLICY === null) {
      trustedTypesPolicy = void 0;
      emptyHTML = "";
    } else {
      if (trustedTypesPolicy === void 0) {
        trustedTypesPolicy = _getDefaultTrustedTypesPolicy();
      }
      if (trustedTypesPolicy && typeof emptyHTML === "string") {
        emptyHTML = _createTrustedHTML("");
      }
    }
    if (freeze) {
      freeze(cfg);
    }
    CONFIG = cfg;
  };
  const ALL_SVG_TAGS = addToSet({}, [...svg$1, ...svgFilters, ...svgDisallowed]);
  const ALL_MATHML_TAGS = addToSet({}, [...mathMl$1, ...mathMlDisallowed]);
  const _checkSvgNamespace = function _checkSvgNamespace2(tagName, parent, parentTagName) {
    if (parent.namespaceURI === HTML_NAMESPACE) {
      return tagName === "svg";
    }
    if (parent.namespaceURI === MATHML_NAMESPACE) {
      return tagName === "svg" && (parentTagName === "annotation-xml" || MATHML_TEXT_INTEGRATION_POINTS[parentTagName]);
    }
    return Boolean(ALL_SVG_TAGS[tagName]);
  };
  const _checkMathMlNamespace = function _checkMathMlNamespace2(tagName, parent, parentTagName) {
    if (parent.namespaceURI === HTML_NAMESPACE) {
      return tagName === "math";
    }
    if (parent.namespaceURI === SVG_NAMESPACE) {
      return tagName === "math" && HTML_INTEGRATION_POINTS[parentTagName];
    }
    return Boolean(ALL_MATHML_TAGS[tagName]);
  };
  const _checkHtmlNamespace = function _checkHtmlNamespace2(tagName, parent, parentTagName) {
    if (parent.namespaceURI === SVG_NAMESPACE && !HTML_INTEGRATION_POINTS[parentTagName]) {
      return false;
    }
    if (parent.namespaceURI === MATHML_NAMESPACE && !MATHML_TEXT_INTEGRATION_POINTS[parentTagName]) {
      return false;
    }
    return !ALL_MATHML_TAGS[tagName] && (COMMON_SVG_AND_HTML_ELEMENTS[tagName] || !ALL_SVG_TAGS[tagName]);
  };
  const _checkValidNamespace = function _checkValidNamespace2(element) {
    let parent = getParentNode(element);
    if (!parent || !parent.tagName) {
      parent = {
        namespaceURI: NAMESPACE,
        tagName: "template"
      };
    }
    const tagName = stringToLowerCase(element.tagName);
    const parentTagName = stringToLowerCase(parent.tagName);
    if (!ALLOWED_NAMESPACES[element.namespaceURI]) {
      return false;
    }
    if (element.namespaceURI === SVG_NAMESPACE) {
      return _checkSvgNamespace(tagName, parent, parentTagName);
    }
    if (element.namespaceURI === MATHML_NAMESPACE) {
      return _checkMathMlNamespace(tagName, parent, parentTagName);
    }
    if (element.namespaceURI === HTML_NAMESPACE) {
      return _checkHtmlNamespace(tagName, parent, parentTagName);
    }
    if (PARSER_MEDIA_TYPE === "application/xhtml+xml" && ALLOWED_NAMESPACES[element.namespaceURI]) {
      return true;
    }
    return false;
  };
  const _forceRemove = function _forceRemove2(node) {
    arrayPush(DOMPurify.removed, {
      element: node
    });
    try {
      getParentNode(node).removeChild(node);
    } catch (_2) {
      remove(node);
      if (!getParentNode(node)) {
        throw typeErrorCreate("a node selected for removal could not be detached from its tree and cannot be safely returned; refusing to sanitize in place");
      }
    }
  };
  const _neutralizeRoot = function _neutralizeRoot2(root2) {
    const childNodes = getChildNodes(root2);
    if (childNodes) {
      const snapshot = [];
      arrayForEach(childNodes, (child) => {
        arrayPush(snapshot, child);
      });
      arrayForEach(snapshot, (child) => {
        try {
          remove(child);
        } catch (_2) {
        }
      });
    }
    const attributes = getAttributes(root2);
    if (attributes) {
      for (let i = attributes.length - 1; i >= 0; --i) {
        const attribute = attributes[i];
        const name = attribute && attribute.name;
        if (typeof name === "string") {
          try {
            root2.removeAttribute(name);
          } catch (_2) {
          }
        }
      }
    }
  };
  const _removeAttribute = function _removeAttribute2(name, element) {
    try {
      arrayPush(DOMPurify.removed, {
        attribute: element.getAttributeNode(name),
        from: element
      });
    } catch (_2) {
      arrayPush(DOMPurify.removed, {
        attribute: null,
        from: element
      });
    }
    element.removeAttribute(name);
    if (name === "is") {
      if (RETURN_DOM || RETURN_DOM_FRAGMENT) {
        try {
          _forceRemove(element);
        } catch (_2) {
        }
      } else {
        try {
          element.setAttribute(name, "");
        } catch (_2) {
        }
      }
    }
  };
  const _stripDisallowedAttributes = function _stripDisallowedAttributes2(element) {
    const attributes = getAttributes(element);
    if (!attributes) {
      return;
    }
    for (let i = attributes.length - 1; i >= 0; --i) {
      const attribute = attributes[i];
      const name = attribute && attribute.name;
      if (typeof name !== "string" || ALLOWED_ATTR[transformCaseFunc(name)]) {
        continue;
      }
      try {
        element.removeAttribute(name);
      } catch (_2) {
      }
    }
  };
  const _neutralizeSubtree = function _neutralizeSubtree2(root2) {
    const stack = [root2];
    while (stack.length > 0) {
      const node = stack.pop();
      const nodeType = getNodeType ? getNodeType(node) : node.nodeType;
      if (nodeType === NODE_TYPE.element) {
        _stripDisallowedAttributes(node);
      }
      const childNodes = getChildNodes(node);
      if (childNodes) {
        for (let i = childNodes.length - 1; i >= 0; --i) {
          stack.push(childNodes[i]);
        }
      }
    }
  };
  const _initDocument = function _initDocument2(dirty) {
    let doc = null;
    let leadingWhitespace = null;
    if (FORCE_BODY) {
      dirty = "<remove></remove>" + dirty;
    } else {
      const matches = stringMatch(dirty, /^[\r\n\t ]+/);
      leadingWhitespace = matches && matches[0];
    }
    if (PARSER_MEDIA_TYPE === "application/xhtml+xml" && NAMESPACE === HTML_NAMESPACE) {
      dirty = '<html xmlns="http://www.w3.org/1999/xhtml"><head></head><body>' + dirty + "</body></html>";
    }
    const dirtyPayload = trustedTypesPolicy ? _createTrustedHTML(dirty) : dirty;
    if (NAMESPACE === HTML_NAMESPACE) {
      try {
        doc = new DOMParser().parseFromString(dirtyPayload, PARSER_MEDIA_TYPE);
      } catch (_2) {
      }
    }
    if (!doc || !doc.documentElement) {
      doc = implementation.createDocument(NAMESPACE, "template", null);
      try {
        doc.documentElement.innerHTML = IS_EMPTY_INPUT ? emptyHTML : dirtyPayload;
      } catch (_2) {
      }
    }
    const body = doc.body || doc.documentElement;
    if (dirty && leadingWhitespace) {
      body.insertBefore(document2.createTextNode(leadingWhitespace), body.childNodes[0] || null);
    }
    if (NAMESPACE === HTML_NAMESPACE) {
      return getElementsByTagName.call(doc, WHOLE_DOCUMENT ? "html" : "body")[0];
    }
    return WHOLE_DOCUMENT ? doc.documentElement : body;
  };
  const _createNodeIterator = function _createNodeIterator2(root2) {
    return createNodeIterator.call(
      root2.ownerDocument || root2,
      root2,
      // eslint-disable-next-line no-bitwise
      NodeFilter.SHOW_ELEMENT | NodeFilter.SHOW_COMMENT | NodeFilter.SHOW_TEXT | NodeFilter.SHOW_PROCESSING_INSTRUCTION | NodeFilter.SHOW_CDATA_SECTION,
      null
    );
  };
  const _stripTemplateExpressions = function _stripTemplateExpressions2(value) {
    value = stringReplace(value, MUSTACHE_EXPR$1, " ");
    value = stringReplace(value, ERB_EXPR$1, " ");
    value = stringReplace(value, TMPLIT_EXPR$1, " ");
    return value;
  };
  const _scrubTemplateExpressions2 = function _scrubTemplateExpressions(node) {
    var _node$querySelectorAl;
    node.normalize();
    const walker = createNodeIterator.call(
      node.ownerDocument || node,
      node,
      // eslint-disable-next-line no-bitwise
      NodeFilter.SHOW_TEXT | NodeFilter.SHOW_COMMENT | NodeFilter.SHOW_CDATA_SECTION | NodeFilter.SHOW_PROCESSING_INSTRUCTION,
      null
    );
    let currentNode = walker.nextNode();
    while (currentNode) {
      currentNode.data = _stripTemplateExpressions(currentNode.data);
      currentNode = walker.nextNode();
    }
    const templates = (_node$querySelectorAl = node.querySelectorAll) === null || _node$querySelectorAl === void 0 ? void 0 : _node$querySelectorAl.call(node, "template");
    if (templates) {
      arrayForEach(templates, (tmpl) => {
        if (_isDocumentFragment(tmpl.content)) {
          _scrubTemplateExpressions2(tmpl.content);
        }
      });
    }
  };
  const _isClobbered = function _isClobbered2(element) {
    const realTagName = getNodeName ? getNodeName(element) : null;
    if (typeof realTagName !== "string") {
      return false;
    }
    if (transformCaseFunc(realTagName) !== "form") {
      return false;
    }
    return typeof element.nodeName !== "string" || typeof element.textContent !== "string" || typeof element.removeChild !== "function" || // Realm-safe NamedNodeMap detection: equality against the cached
    // prototype getter. Clobbered .attributes (e.g. <input name="attributes">)
    // makes the direct read diverge from the cached read; a clean form
    // (same-realm OR foreign-realm) has both reads pointing at the same
    // canonical NamedNodeMap.
    element.attributes !== getAttributes(element) || typeof element.removeAttribute !== "function" || typeof element.setAttribute !== "function" || typeof element.namespaceURI !== "string" || typeof element.insertBefore !== "function" || typeof element.hasChildNodes !== "function" || // NodeType clobbering probe. Cached Node.prototype.nodeType getter
    // returns the integer 1 for any Element regardless of realm; direct
    // read on a clobbered form (e.g. <input name="nodeType">) returns
    // the named child element. Cheap addition — nodeType is read from
    // an internal slot, no serialization cost — and removes a residual
    // clobbering surface used by several mXSS / PI / comment branches
    // in _sanitizeElements that compare currentNode.nodeType directly.
    element.nodeType !== getNodeType(element) || // HTMLFormElement has [LegacyOverrideBuiltIns]: a descendant named
    // "childNodes" shadows the prototype getter. Direct reads of
    // form.childNodes from a clobbered form return the named child
    // instead of the real NodeList, so any walk that reads it directly
    // skips the form's real children. Compare the direct read to the
    // cached Node.prototype getter — when the form's named-property
    // getter intercepts the read, the two values differ and we flag
    // the form. This catches every clobbering child type (input,
    // select, etc.) regardless of whether the named child happens to
    // carry a numeric .length, which a typeof-based probe would miss
    // (e.g. HTMLSelectElement.length is a defined unsigned-long).
    element.childNodes !== getChildNodes(element);
  };
  const _isDocumentFragment = function _isDocumentFragment2(value) {
    if (!getNodeType || typeof value !== "object" || value === null) {
      return false;
    }
    try {
      return getNodeType(value) === NODE_TYPE.documentFragment;
    } catch (_2) {
      return false;
    }
  };
  const _isNode = function _isNode2(value) {
    if (!getNodeType || typeof value !== "object" || value === null) {
      return false;
    }
    try {
      return typeof getNodeType(value) === "number";
    } catch (_2) {
      return false;
    }
  };
  function _executeHooks(hooks2, currentNode, data) {
    if (hooks2.length === 0) {
      return;
    }
    arrayForEach(hooks2, (hook) => {
      hook.call(DOMPurify, currentNode, data, CONFIG);
    });
  }
  const _isUnsafeNode = function _isUnsafeNode2(currentNode, tagName) {
    if (SAFE_FOR_XML && currentNode.hasChildNodes() && !_isNode(currentNode.firstElementChild) && regExpTest(ELEMENT_MARKUP_PROBE, currentNode.textContent) && regExpTest(ELEMENT_MARKUP_PROBE, currentNode.innerHTML)) {
      return true;
    }
    if (SAFE_FOR_XML && currentNode.namespaceURI === HTML_NAMESPACE && tagName === "style" && _isNode(currentNode.firstElementChild)) {
      return true;
    }
    if (currentNode.nodeType === NODE_TYPE.processingInstruction) {
      return true;
    }
    if (SAFE_FOR_XML && currentNode.nodeType === NODE_TYPE.comment && regExpTest(COMMENT_MARKUP_PROBE, currentNode.data)) {
      return true;
    }
    return false;
  };
  const _sanitizeDisallowedNode = function _sanitizeDisallowedNode2(currentNode, tagName) {
    if (!FORBID_TAGS[tagName] && _isBasicCustomElement(tagName)) {
      if (CUSTOM_ELEMENT_HANDLING.tagNameCheck instanceof RegExp && regExpTest(CUSTOM_ELEMENT_HANDLING.tagNameCheck, tagName)) {
        return false;
      }
      if (CUSTOM_ELEMENT_HANDLING.tagNameCheck instanceof Function && CUSTOM_ELEMENT_HANDLING.tagNameCheck(tagName)) {
        return false;
      }
    }
    if (KEEP_CONTENT && !FORBID_CONTENTS[tagName]) {
      const parentNode = getParentNode(currentNode);
      const childNodes = getChildNodes(currentNode);
      if (childNodes && parentNode) {
        const childCount = childNodes.length;
        for (let i = childCount - 1; i >= 0; --i) {
          const hoisted = IN_PLACE ? childNodes[i] : cloneNode(childNodes[i], true);
          parentNode.insertBefore(hoisted, getNextSibling(currentNode));
        }
      }
    }
    _forceRemove(currentNode);
    return true;
  };
  const _sanitizeElements = function _sanitizeElements2(currentNode) {
    _executeHooks(hooks.beforeSanitizeElements, currentNode, null);
    if (_isClobbered(currentNode)) {
      _forceRemove(currentNode);
      return true;
    }
    const tagName = transformCaseFunc(getNodeName ? getNodeName(currentNode) : currentNode.nodeName);
    _executeHooks(hooks.uponSanitizeElement, currentNode, {
      tagName,
      allowedTags: ALLOWED_TAGS
    });
    if (_isUnsafeNode(currentNode, tagName)) {
      _forceRemove(currentNode);
      return true;
    }
    if (FORBID_TAGS[tagName] || !(EXTRA_ELEMENT_HANDLING.tagCheck instanceof Function && EXTRA_ELEMENT_HANDLING.tagCheck(tagName)) && !ALLOWED_TAGS[tagName]) {
      return _sanitizeDisallowedNode(currentNode, tagName);
    }
    const nt2 = getNodeType ? getNodeType(currentNode) : currentNode.nodeType;
    if (nt2 === NODE_TYPE.element && !_checkValidNamespace(currentNode)) {
      _forceRemove(currentNode);
      return true;
    }
    if ((tagName === "noscript" || tagName === "noembed" || tagName === "noframes") && regExpTest(FALLBACK_TAG_CLOSE, currentNode.innerHTML)) {
      _forceRemove(currentNode);
      return true;
    }
    if (SAFE_FOR_TEMPLATES && currentNode.nodeType === NODE_TYPE.text) {
      const content = _stripTemplateExpressions(currentNode.textContent);
      if (currentNode.textContent !== content) {
        arrayPush(DOMPurify.removed, {
          element: currentNode.cloneNode()
        });
        currentNode.textContent = content;
      }
    }
    _executeHooks(hooks.afterSanitizeElements, currentNode, null);
    return false;
  };
  const _isValidAttribute = function _isValidAttribute2(lcTag, lcName, value) {
    if (FORBID_ATTR[lcName]) {
      return false;
    }
    if (SANITIZE_DOM && (lcName === "id" || lcName === "name") && (value in document2 || value in formElement)) {
      return false;
    }
    const nameIsPermitted = ALLOWED_ATTR[lcName] || EXTRA_ELEMENT_HANDLING.attributeCheck instanceof Function && EXTRA_ELEMENT_HANDLING.attributeCheck(lcName, lcTag);
    if (ALLOW_DATA_ATTR && regExpTest(DATA_ATTR$1, lcName)) ;
    else if (ALLOW_ARIA_ATTR && regExpTest(ARIA_ATTR$1, lcName)) ;
    else if (!nameIsPermitted) {
      if (
        // First condition does a very basic check if a) it's basically a valid custom element tagname AND
        // b) if the tagName passes whatever the user has configured for CUSTOM_ELEMENT_HANDLING.tagNameCheck
        // and c) if the attribute name passes whatever the user has configured for CUSTOM_ELEMENT_HANDLING.attributeNameCheck
        _isBasicCustomElement(lcTag) && (CUSTOM_ELEMENT_HANDLING.tagNameCheck instanceof RegExp && regExpTest(CUSTOM_ELEMENT_HANDLING.tagNameCheck, lcTag) || CUSTOM_ELEMENT_HANDLING.tagNameCheck instanceof Function && CUSTOM_ELEMENT_HANDLING.tagNameCheck(lcTag)) && (CUSTOM_ELEMENT_HANDLING.attributeNameCheck instanceof RegExp && regExpTest(CUSTOM_ELEMENT_HANDLING.attributeNameCheck, lcName) || CUSTOM_ELEMENT_HANDLING.attributeNameCheck instanceof Function && CUSTOM_ELEMENT_HANDLING.attributeNameCheck(lcName, lcTag)) || // Alternative, second condition checks if it's an `is`-attribute, AND
        // the value passes whatever the user has configured for CUSTOM_ELEMENT_HANDLING.tagNameCheck
        lcName === "is" && CUSTOM_ELEMENT_HANDLING.allowCustomizedBuiltInElements && (CUSTOM_ELEMENT_HANDLING.tagNameCheck instanceof RegExp && regExpTest(CUSTOM_ELEMENT_HANDLING.tagNameCheck, value) || CUSTOM_ELEMENT_HANDLING.tagNameCheck instanceof Function && CUSTOM_ELEMENT_HANDLING.tagNameCheck(value))
      ) ;
      else {
        return false;
      }
    } else if (URI_SAFE_ATTRIBUTES[lcName]) ;
    else if (regExpTest(IS_ALLOWED_URI$1, stringReplace(value, ATTR_WHITESPACE$1, ""))) ;
    else if ((lcName === "src" || lcName === "xlink:href" || lcName === "href") && lcTag !== "script" && stringIndexOf(value, "data:") === 0 && DATA_URI_TAGS[lcTag]) ;
    else if (ALLOW_UNKNOWN_PROTOCOLS && !regExpTest(IS_SCRIPT_OR_DATA$1, stringReplace(value, ATTR_WHITESPACE$1, ""))) ;
    else if (value) {
      return false;
    } else ;
    return true;
  };
  const RESERVED_CUSTOM_ELEMENT_NAMES = addToSet({}, ["annotation-xml", "color-profile", "font-face", "font-face-format", "font-face-name", "font-face-src", "font-face-uri", "missing-glyph"]);
  const _isBasicCustomElement = function _isBasicCustomElement2(tagName) {
    return !RESERVED_CUSTOM_ELEMENT_NAMES[stringToLowerCase(tagName)] && regExpTest(CUSTOM_ELEMENT$1, tagName);
  };
  const _applyTrustedTypesToAttribute = function _applyTrustedTypesToAttribute2(lcTag, lcName, namespaceURI, value) {
    if (trustedTypesPolicy && typeof trustedTypes === "object" && typeof trustedTypes.getAttributeType === "function" && !namespaceURI) {
      switch (trustedTypes.getAttributeType(lcTag, lcName)) {
        case "TrustedHTML": {
          return _createTrustedHTML(value);
        }
        case "TrustedScriptURL": {
          return _createTrustedScriptURL(value);
        }
      }
    }
    return value;
  };
  const _setAttributeValue = function _setAttributeValue2(currentNode, name, namespaceURI, value) {
    try {
      if (namespaceURI) {
        currentNode.setAttributeNS(namespaceURI, name, value);
      } else {
        currentNode.setAttribute(name, value);
      }
      if (_isClobbered(currentNode)) {
        _forceRemove(currentNode);
      } else {
        arrayPop(DOMPurify.removed);
      }
    } catch (_2) {
      _removeAttribute(name, currentNode);
    }
  };
  const _sanitizeAttributes = function _sanitizeAttributes2(currentNode) {
    _executeHooks(hooks.beforeSanitizeAttributes, currentNode, null);
    const attributes = currentNode.attributes;
    if (!attributes || _isClobbered(currentNode)) {
      return;
    }
    const hookEvent = {
      attrName: "",
      attrValue: "",
      keepAttr: true,
      allowedAttributes: ALLOWED_ATTR,
      forceKeepAttr: void 0
    };
    let l3 = attributes.length;
    const lcTag = transformCaseFunc(currentNode.nodeName);
    while (l3--) {
      const attr = attributes[l3];
      const name = attr.name, namespaceURI = attr.namespaceURI, attrValue = attr.value;
      const lcName = transformCaseFunc(name);
      const initValue = attrValue;
      let value = name === "value" ? initValue : stringTrim(initValue);
      hookEvent.attrName = lcName;
      hookEvent.attrValue = value;
      hookEvent.keepAttr = true;
      hookEvent.forceKeepAttr = void 0;
      _executeHooks(hooks.uponSanitizeAttribute, currentNode, hookEvent);
      value = hookEvent.attrValue;
      if (SANITIZE_NAMED_PROPS && (lcName === "id" || lcName === "name") && stringIndexOf(value, SANITIZE_NAMED_PROPS_PREFIX) !== 0) {
        _removeAttribute(name, currentNode);
        value = SANITIZE_NAMED_PROPS_PREFIX + value;
      }
      if (SAFE_FOR_XML && regExpTest(/((--!?|])>)|<\/(style|script|title|xmp|textarea|noscript|iframe|noembed|noframes)/i, value)) {
        _removeAttribute(name, currentNode);
        continue;
      }
      if (lcName === "attributename" && stringMatch(value, "href")) {
        _removeAttribute(name, currentNode);
        continue;
      }
      if (hookEvent.forceKeepAttr) {
        continue;
      }
      if (!hookEvent.keepAttr) {
        _removeAttribute(name, currentNode);
        continue;
      }
      if (!ALLOW_SELF_CLOSE_IN_ATTR && regExpTest(SELF_CLOSING_TAG, value)) {
        _removeAttribute(name, currentNode);
        continue;
      }
      if (SAFE_FOR_TEMPLATES) {
        value = _stripTemplateExpressions(value);
      }
      if (!_isValidAttribute(lcTag, lcName, value)) {
        _removeAttribute(name, currentNode);
        continue;
      }
      value = _applyTrustedTypesToAttribute(lcTag, lcName, namespaceURI, value);
      if (value !== initValue) {
        _setAttributeValue(currentNode, name, namespaceURI, value);
      }
    }
    _executeHooks(hooks.afterSanitizeAttributes, currentNode, null);
  };
  const _sanitizeShadowDOM2 = function _sanitizeShadowDOM(fragment) {
    let shadowNode = null;
    const shadowIterator = _createNodeIterator(fragment);
    _executeHooks(hooks.beforeSanitizeShadowDOM, fragment, null);
    while (shadowNode = shadowIterator.nextNode()) {
      _executeHooks(hooks.uponSanitizeShadowNode, shadowNode, null);
      _sanitizeElements(shadowNode);
      _sanitizeAttributes(shadowNode);
      if (_isDocumentFragment(shadowNode.content)) {
        _sanitizeShadowDOM2(shadowNode.content);
      }
      const shadowNodeType = getNodeType ? getNodeType(shadowNode) : shadowNode.nodeType;
      if (shadowNodeType === NODE_TYPE.element) {
        const innerSr = getShadowRoot(shadowNode);
        if (_isDocumentFragment(innerSr)) {
          _sanitizeAttachedShadowRoots(innerSr);
          _sanitizeShadowDOM2(innerSr);
        }
      }
    }
    _executeHooks(hooks.afterSanitizeShadowDOM, fragment, null);
  };
  const _sanitizeAttachedShadowRoots = function _sanitizeAttachedShadowRoots2(root2) {
    const stack = [{
      node: root2,
      shadow: null
    }];
    while (stack.length > 0) {
      const item = stack.pop();
      if (item.shadow) {
        _sanitizeShadowDOM2(item.shadow);
        continue;
      }
      const node = item.node;
      const nodeType = getNodeType ? getNodeType(node) : node.nodeType;
      const isElement = nodeType === NODE_TYPE.element;
      const childNodes = getChildNodes(node);
      if (childNodes) {
        for (let i = childNodes.length - 1; i >= 0; --i) {
          stack.push({
            node: childNodes[i],
            shadow: null
          });
        }
      }
      if (isElement) {
        const rootName = getNodeName ? getNodeName(node) : null;
        if (typeof rootName === "string" && transformCaseFunc(rootName) === "template") {
          const content = node.content;
          if (_isDocumentFragment(content)) {
            stack.push({
              node: content,
              shadow: null
            });
          }
        }
      }
      if (isElement) {
        const sr = getShadowRoot(node);
        if (_isDocumentFragment(sr)) {
          stack.push({
            node: null,
            shadow: sr
          }, {
            node: sr,
            shadow: null
          });
        }
      }
    }
  };
  DOMPurify.sanitize = function(dirty) {
    let cfg = arguments.length > 1 && arguments[1] !== void 0 ? arguments[1] : {};
    let body = null;
    let importedNode = null;
    let currentNode = null;
    let returnNode = null;
    IS_EMPTY_INPUT = !dirty;
    if (IS_EMPTY_INPUT) {
      dirty = "<!-->";
    }
    if (typeof dirty !== "string" && !_isNode(dirty)) {
      dirty = stringifyValue(dirty);
      if (typeof dirty !== "string") {
        throw typeErrorCreate("dirty is not a string, aborting");
      }
    }
    if (!DOMPurify.isSupported) {
      return dirty;
    }
    if (SET_CONFIG) {
      ALLOWED_TAGS = SET_CONFIG_ALLOWED_TAGS;
      ALLOWED_ATTR = SET_CONFIG_ALLOWED_ATTR;
    } else {
      _parseConfig(cfg);
    }
    if (hooks.uponSanitizeElement.length > 0 || hooks.uponSanitizeAttribute.length > 0) {
      ALLOWED_TAGS = clone(ALLOWED_TAGS);
    }
    if (hooks.uponSanitizeAttribute.length > 0) {
      ALLOWED_ATTR = clone(ALLOWED_ATTR);
    }
    DOMPurify.removed = [];
    const inPlace = IN_PLACE && typeof dirty !== "string" && _isNode(dirty);
    if (inPlace) {
      const nn = getNodeName ? getNodeName(dirty) : dirty.nodeName;
      if (typeof nn === "string") {
        const tagName = transformCaseFunc(nn);
        if (!ALLOWED_TAGS[tagName] || FORBID_TAGS[tagName]) {
          throw typeErrorCreate("root node is forbidden and cannot be sanitized in-place");
        }
      }
      if (_isClobbered(dirty)) {
        throw typeErrorCreate("root node is clobbered and cannot be sanitized in-place");
      }
      try {
        _sanitizeAttachedShadowRoots(dirty);
      } catch (error) {
        _neutralizeRoot(dirty);
        throw error;
      }
    } else if (_isNode(dirty)) {
      body = _initDocument("<!---->");
      importedNode = body.ownerDocument.importNode(dirty, true);
      if (importedNode.nodeType === NODE_TYPE.element && importedNode.nodeName === "BODY") {
        body = importedNode;
      } else if (importedNode.nodeName === "HTML") {
        body = importedNode;
      } else {
        body.appendChild(importedNode);
      }
      _sanitizeAttachedShadowRoots(importedNode);
    } else {
      if (!RETURN_DOM && !SAFE_FOR_TEMPLATES && !WHOLE_DOCUMENT && // eslint-disable-next-line unicorn/prefer-includes
      dirty.indexOf("<") === -1) {
        return trustedTypesPolicy && RETURN_TRUSTED_TYPE ? _createTrustedHTML(dirty) : dirty;
      }
      body = _initDocument(dirty);
      if (!body) {
        return RETURN_DOM ? null : RETURN_TRUSTED_TYPE ? emptyHTML : "";
      }
    }
    if (body && FORCE_BODY) {
      _forceRemove(body.firstChild);
    }
    const nodeIterator = _createNodeIterator(inPlace ? dirty : body);
    try {
      while (currentNode = nodeIterator.nextNode()) {
        _sanitizeElements(currentNode);
        _sanitizeAttributes(currentNode);
        if (_isDocumentFragment(currentNode.content)) {
          _sanitizeShadowDOM2(currentNode.content);
        }
      }
    } catch (error) {
      if (inPlace) {
        _neutralizeRoot(dirty);
      }
      throw error;
    }
    if (inPlace) {
      arrayForEach(DOMPurify.removed, (entry) => {
        if (entry.element) {
          _neutralizeSubtree(entry.element);
        }
      });
      if (SAFE_FOR_TEMPLATES) {
        _scrubTemplateExpressions2(dirty);
      }
      return dirty;
    }
    if (RETURN_DOM) {
      if (SAFE_FOR_TEMPLATES) {
        _scrubTemplateExpressions2(body);
      }
      if (RETURN_DOM_FRAGMENT) {
        returnNode = createDocumentFragment.call(body.ownerDocument);
        while (body.firstChild) {
          returnNode.appendChild(body.firstChild);
        }
      } else {
        returnNode = body;
      }
      if (ALLOWED_ATTR.shadowroot || ALLOWED_ATTR.shadowrootmode) {
        returnNode = importNode.call(originalDocument, returnNode, true);
      }
      return returnNode;
    }
    let serializedHTML = WHOLE_DOCUMENT ? body.outerHTML : body.innerHTML;
    if (WHOLE_DOCUMENT && ALLOWED_TAGS["!doctype"] && body.ownerDocument && body.ownerDocument.doctype && body.ownerDocument.doctype.name && regExpTest(DOCTYPE_NAME, body.ownerDocument.doctype.name)) {
      serializedHTML = "<!DOCTYPE " + body.ownerDocument.doctype.name + ">\n" + serializedHTML;
    }
    if (SAFE_FOR_TEMPLATES) {
      serializedHTML = _stripTemplateExpressions(serializedHTML);
    }
    return trustedTypesPolicy && RETURN_TRUSTED_TYPE ? _createTrustedHTML(serializedHTML) : serializedHTML;
  };
  DOMPurify.setConfig = function() {
    let cfg = arguments.length > 0 && arguments[0] !== void 0 ? arguments[0] : {};
    _parseConfig(cfg);
    SET_CONFIG = true;
    SET_CONFIG_ALLOWED_TAGS = ALLOWED_TAGS;
    SET_CONFIG_ALLOWED_ATTR = ALLOWED_ATTR;
  };
  DOMPurify.clearConfig = function() {
    CONFIG = null;
    SET_CONFIG = false;
    SET_CONFIG_ALLOWED_TAGS = null;
    SET_CONFIG_ALLOWED_ATTR = null;
    trustedTypesPolicy = defaultTrustedTypesPolicy;
    emptyHTML = "";
  };
  DOMPurify.isValidAttribute = function(tag, attr, value) {
    if (!CONFIG) {
      _parseConfig({});
    }
    const lcTag = transformCaseFunc(tag);
    const lcName = transformCaseFunc(attr);
    return _isValidAttribute(lcTag, lcName, value);
  };
  DOMPurify.addHook = function(entryPoint, hookFunction) {
    if (typeof hookFunction !== "function") {
      return;
    }
    if (!objectHasOwnProperty(hooks, entryPoint)) {
      return;
    }
    arrayPush(hooks[entryPoint], hookFunction);
  };
  DOMPurify.removeHook = function(entryPoint, hookFunction) {
    if (!objectHasOwnProperty(hooks, entryPoint)) {
      return void 0;
    }
    if (hookFunction !== void 0) {
      const index = arrayLastIndexOf(hooks[entryPoint], hookFunction);
      return index === -1 ? void 0 : arraySplice(hooks[entryPoint], index, 1)[0];
    }
    return arrayPop(hooks[entryPoint]);
  };
  DOMPurify.removeHooks = function(entryPoint) {
    if (!objectHasOwnProperty(hooks, entryPoint)) {
      return;
    }
    hooks[entryPoint] = [];
  };
  DOMPurify.removeAllHooks = function() {
    hooks = _createHooksMap();
  };
  return DOMPurify;
}
var purify = createDOMPurify();
function M() {
  return { async: false, breaks: false, extensions: null, gfm: true, hooks: null, pedantic: false, renderer: null, silent: false, tokenizer: null, walkTokens: null };
}
var T = M();
function N(l3) {
  T = l3;
}
var _ = { exec: () => null };
function E(l3) {
  let e = [];
  return (t) => {
    let n = Math.max(0, Math.min(3, t - 1)), s = e[n];
    return s || (s = l3(n), e[n] = s), s;
  };
}
function d(l3, e = "") {
  let t = typeof l3 == "string" ? l3 : l3.source, n = { replace: (s, r) => {
    let i = typeof r == "string" ? r : r.source;
    return i = i.replace(m.caret, "$1"), t = t.replace(s, i), n;
  }, getRegex: () => new RegExp(t, e) };
  return n;
}
var Te = ((l3 = "") => {
  try {
    return !!new RegExp("(?<=1)(?<!1)" + l3);
  } catch {
    return false;
  }
})(), m = { codeRemoveIndent: /^(?: {1,4}| {0,3}\t)/gm, outputLinkReplace: /\\([\[\]])/g, indentCodeCompensation: /^(\s+)(?:```)/, beginningSpace: /^\s+/, endingHash: /#$/, startingSpaceChar: /^ /, endingSpaceChar: / $/, nonSpaceChar: /[^ ]/, newLineCharGlobal: /\n/g, tabCharGlobal: /\t/g, multipleSpaceGlobal: /\s+/g, blankLine: /^[ \t]*$/, doubleBlankLine: /\n[ \t]*\n[ \t]*$/, blockquoteStart: /^ {0,3}>/, blockquoteSetextReplace: /\n {0,3}((?:=+|-+) *)(?=\n|$)/g, blockquoteSetextReplace2: /^ {0,3}>[ \t]?/gm, listReplaceNesting: /^ {1,4}(?=( {4})*[^ ])/g, listIsTask: /^\[[ xX]\] +\S/, listReplaceTask: /^\[[ xX]\] +/, listTaskCheckbox: /\[[ xX]\]/, anyLine: /\n.*\n/, hrefBrackets: /^<(.*)>$/, tableDelimiter: /[:|]/, tableAlignChars: /^\||\| *$/g, tableRowBlankLine: /\n[ \t]*$/, tableAlignRight: /^ *-+: *$/, tableAlignCenter: /^ *:-+: *$/, tableAlignLeft: /^ *:-+ *$/, startATag: /^<a /i, endATag: /^<\/a>/i, startPreScriptTag: /^<(pre|code|kbd|script)(\s|>)/i, endPreScriptTag: /^<\/(pre|code|kbd|script)(\s|>)/i, startAngleBracket: /^</, endAngleBracket: />$/, pedanticHrefTitle: /^([^'"]*[^\s])\s+(['"])(.*)\2/, unicodeAlphaNumeric: /[\p{L}\p{N}]/u, escapeTest: /[&<>"']/, escapeReplace: /[&<>"']/g, escapeTestNoEncode: /[<>"']|&(?!(#\d{1,7}|#[Xx][a-fA-F0-9]{1,6}|\w+);)/, escapeReplaceNoEncode: /[<>"']|&(?!(#\d{1,7}|#[Xx][a-fA-F0-9]{1,6}|\w+);)/g, caret: /(^|[^\[])\^/g, percentDecode: /%25/g, findPipe: /\|/g, splitPipe: / \|/, slashPipe: /\\\|/g, carriageReturn: /\r\n|\r/g, spaceLine: /^ +$/gm, notSpaceStart: /^\S*/, endingNewline: /\n$/, listItemRegex: (l3) => new RegExp(`^( {0,3}${l3})((?:[	 ][^\\n]*)?(?:\\n|$))`), nextBulletRegex: E((l3) => new RegExp(`^ {0,${l3}}(?:[*+-]|\\d{1,9}[.)])((?:[ 	][^\\n]*)?(?:\\n|$))`)), hrRegex: E((l3) => new RegExp(`^ {0,${l3}}((?:- *){3,}|(?:_ *){3,}|(?:\\* *){3,})(?:\\n+|$)`)), fencesBeginRegex: E((l3) => new RegExp(`^ {0,${l3}}(?:\`\`\`|~~~)`)), headingBeginRegex: E((l3) => new RegExp(`^ {0,${l3}}#`)), htmlBeginRegex: E((l3) => new RegExp(`^ {0,${l3}}<(?:[a-z].*>|!--)`, "i")), blockquoteBeginRegex: E((l3) => new RegExp(`^ {0,${l3}}>`)) }, Oe = /^(?:[ \t]*(?:\n|$))+/, we = /^((?: {4}| {0,3}\t)[^\n]+(?:\n(?:[ \t]*(?:\n|$))*)?)+/, ye = /^ {0,3}(`{3,}(?=[^`\n]*(?:\n|$))|~{3,})([^\n]*)(?:\n|$)(?:|([\s\S]*?)(?:\n|$))(?: {0,3}\1[~`]* *(?=\n|$)|$)/, B = /^ {0,3}((?:-[\t ]*){3,}|(?:_[ \t]*){3,}|(?:\*[ \t]*){3,})(?:\n+|$)/, Pe = /^ {0,3}(#{1,6})(?=\s|$)(.*)(?:\n+|$)/, j = / {0,3}(?:[*+-]|\d{1,9}[.)])/, oe = /^(?!bull |blockCode|fences|blockquote|heading|html|table)((?:.|\n(?!\s*?\n|bull |blockCode|fences|blockquote|heading|html|table))+?)\n {0,3}(=+|-+) *(?:\n+|$)/, ae = d(oe).replace(/bull/g, j).replace(/blockCode/g, /(?: {4}| {0,3}\t)/).replace(/fences/g, / {0,3}(?:`{3,}|~{3,})/).replace(/blockquote/g, / {0,3}>/).replace(/heading/g, / {0,3}#{1,6}/).replace(/html/g, / {0,3}<[^\n>]+>\n/).replace(/\|table/g, "").getRegex(), Se = d(oe).replace(/bull/g, j).replace(/blockCode/g, /(?: {4}| {0,3}\t)/).replace(/fences/g, / {0,3}(?:`{3,}|~{3,})/).replace(/blockquote/g, / {0,3}>/).replace(/heading/g, / {0,3}#{1,6}/).replace(/html/g, / {0,3}<[^\n>]+>\n/).replace(/table/g, / {0,3}\|?(?:[:\- ]*\|)+[\:\- ]*\n/).getRegex(), F = /^([^\n]+(?:\n(?!hr|heading|lheading|blockquote|fences|list|html|table| +\n)[^\n]+)*)/, $e = /^[^\n]+/, U = /(?!\s*\])(?:\\[\s\S]|[^\[\]\\])+/, Le = d(/^ {0,3}\[(label)\]: *(?:\n[ \t]*)?([^<\s][^\s]*|<.*?>)(?:(?: +(?:\n[ \t]*)?| *\n[ \t]*)(title))? *(?:\n+|$)/).replace("label", U).replace("title", /(?:"(?:\\"?|[^"\\])*"|'[^'\n]*(?:\n[^'\n]+)*\n?'|\([^()]*\))/).getRegex(), _e = d(/^(bull)([ \t][^\n]*?)?(?:\n|$)/).replace(/bull/g, j).getRegex(), H = "address|article|aside|base|basefont|blockquote|body|caption|center|col|colgroup|dd|details|dialog|dir|div|dl|dt|fieldset|figcaption|figure|footer|form|frame|frameset|h[1-6]|head|header|hr|html|iframe|legend|li|link|main|menu|menuitem|meta|nav|noframes|ol|optgroup|option|p|param|search|section|summary|table|tbody|td|tfoot|th|thead|title|tr|track|ul", K = /<!--(?:-?>|[\s\S]*?(?:-->|$))/, ze = d("^ {0,3}(?:<(script|pre|style|textarea)[\\s>][\\s\\S]*?(?:</\\1>[^\\n]*\\n+|$)|comment[^\\n]*(\\n+|$)|<\\?[\\s\\S]*?(?:\\?>\\n*|$)|<![A-Z][\\s\\S]*?(?:>\\n*|$)|<!\\[CDATA\\[[\\s\\S]*?(?:\\]\\]>\\n*|$)|</?(tag)(?: +|\\n|/?>)[\\s\\S]*?(?:(?:\\n[ 	]*)+\\n|$)|<(?!script|pre|style|textarea)([a-z][\\w-]*)(?:attribute)*? */?>(?=[ \\t]*(?:\\n|$))[\\s\\S]*?(?:(?:\\n[ 	]*)+\\n|$)|</(?!script|pre|style|textarea)[a-z][\\w-]*\\s*>(?=[ \\t]*(?:\\n|$))[\\s\\S]*?(?:(?:\\n[ 	]*)+\\n|$))", "i").replace("comment", K).replace("tag", H).replace("attribute", / +[a-zA-Z:_][\w.:-]*(?: *= *"[^"\n]*"| *= *'[^'\n]*'| *= *[^\s"'=<>`]+)?/).getRegex(), le = d(F).replace("hr", B).replace("heading", " {0,3}#{1,6}(?:\\s|$)").replace("|lheading", "").replace("|table", "").replace("blockquote", " {0,3}>").replace("fences", " {0,3}(?:`{3,}(?=[^`\\n]*\\n)|~{3,})[^\\n]*\\n").replace("list", " {0,3}(?:[*+-]|1[.)])[ \\t]+[^ \\t\\n]").replace("html", "</?(?:tag)(?: +|\\n|/?>)|<(?:script|pre|style|textarea|!--)").replace("tag", H).getRegex(), Me = d(/^( {0,3}> ?(paragraph|[^\n]*)(?:\n|$))+/).replace("paragraph", le).getRegex(), W = { blockquote: Me, code: we, def: Le, fences: ye, heading: Pe, hr: B, html: ze, lheading: ae, list: _e, newline: Oe, paragraph: le, table: _, text: $e }, se = d("^ *([^\\n ].*)\\n {0,3}((?:\\| *)?:?-+:? *(?:\\| *:?-+:? *)*(?:\\| *)?)(?:\\n((?:(?! *\\n|hr|heading|blockquote|code|fences|list|html).*(?:\\n|$))*)\\n*|$)").replace("hr", B).replace("heading", " {0,3}#{1,6}(?:\\s|$)").replace("blockquote", " {0,3}>").replace("code", "(?: {4}| {0,3}	)[^\\n]").replace("fences", " {0,3}(?:`{3,}(?=[^`\\n]*\\n)|~{3,})[^\\n]*\\n").replace("list", " {0,3}(?:[*+-]|1[.)])[ \\t]").replace("html", "</?(?:tag)(?: +|\\n|/?>)|<(?:script|pre|style|textarea|!--)").replace("tag", H).getRegex(), Ee = { ...W, lheading: Se, table: se, paragraph: d(F).replace("hr", B).replace("heading", " {0,3}#{1,6}(?:\\s|$)").replace("|lheading", "").replace("table", se).replace("blockquote", " {0,3}>").replace("fences", " {0,3}(?:`{3,}(?=[^`\\n]*\\n)|~{3,})[^\\n]*\\n").replace("list", " {0,3}(?:[*+-]|1[.)])[ \\t]+[^ \\t\\n]").replace("html", "</?(?:tag)(?: +|\\n|/?>)|<(?:script|pre|style|textarea|!--)").replace("tag", H).getRegex() }, Ie = { ...W, html: d(`^ *(?:comment *(?:\\n|\\s*$)|<(tag)[\\s\\S]+?</\\1> *(?:\\n{2,}|\\s*$)|<tag(?:"[^"]*"|'[^']*'|\\s[^'"/>\\s]*)*?/?> *(?:\\n{2,}|\\s*$))`).replace("comment", K).replace(/tag/g, "(?!(?:a|em|strong|small|s|cite|q|dfn|abbr|data|time|code|var|samp|kbd|sub|sup|i|b|u|mark|ruby|rt|rp|bdi|bdo|span|br|wbr|ins|del|img)\\b)\\w+(?!:|[^\\w\\s@]*@)\\b").getRegex(), def: /^ *\[([^\]]+)\]: *<?([^\s>]+)>?(?: +(["(][^\n]+[")]))? *(?:\n+|$)/, heading: /^(#{1,6})(.*)(?:\n+|$)/, fences: _, lheading: /^(.+?)\n {0,3}(=+|-+) *(?:\n+|$)/, paragraph: d(F).replace("hr", B).replace("heading", ` *#{1,6} *[^
]`).replace("lheading", ae).replace("|table", "").replace("blockquote", " {0,3}>").replace("|fences", "").replace("|list", "").replace("|html", "").replace("|tag", "").getRegex() }, Ae = /^\\([!"#$%&'()*+,\-./:;<=>?@\[\]\\^_`{|}~])/, Ce = /^(`+)([^`]|[^`][\s\S]*?[^`])\1(?!`)/, ue = /^( {2,}|\\)\n(?!\s*$)/, Be = /^(`+|[^`])(?:(?= {2,}\n)|[\s\S]*?(?:(?=[\\<!\[`*_]|\b_|$)|[^ ](?= {2,}\n)))/, I = /[\p{P}\p{S}]/u, Z = /[\s\p{P}\p{S}]/u, X = /[^\s\p{P}\p{S}]/u, De = d(/^((?![*_])punctSpace)/, "u").replace(/punctSpace/g, Z).getRegex(), pe = /(?!~)[\p{P}\p{S}]/u, qe = /(?!~)[\s\p{P}\p{S}]/u, ve = /(?:[^\s\p{P}\p{S}]|~)/u, He = d(/link|precode-code|html/, "g").replace("link", /\[(?:[^\[\]`]|(?<a>`+)[^`]+\k<a>(?!`))*?\]\((?:\\[\s\S]|[^\\\(\)]|\((?:\\[\s\S]|[^\\\(\)])*\))*\)/).replace("precode-", Te ? "(?<!`)()" : "(^^|[^`])").replace("code", /(?<b>`+)[^`]+\k<b>(?!`)/).replace("html", /<(?! )[^<>]*?>/).getRegex(), ce = /^(?:\*+(?:((?!\*)punct)|([^\s*]))?)|^_+(?:((?!_)punct)|([^\s_]))?/, Ze = d(ce, "u").replace(/punct/g, I).getRegex(), Ge = d(ce, "u").replace(/punct/g, pe).getRegex(), he = "^[^_*]*?__[^_*]*?\\*[^_*]*?(?=__)|[^*]+(?=[^*])|(?!\\*)punct(\\*+)(?=[\\s]|$)|notPunctSpace(\\*+)(?!\\*)(?=punctSpace|$)|(?!\\*)punctSpace(\\*+)(?=notPunctSpace)|[\\s](\\*+)(?!\\*)(?=punct)|(?!\\*)punct(\\*+)(?!\\*)(?=punct)|notPunctSpace(\\*+)(?=notPunctSpace)", Ne = d(he, "gu").replace(/notPunctSpace/g, X).replace(/punctSpace/g, Z).replace(/punct/g, I).getRegex(), Qe = d(he, "gu").replace(/notPunctSpace/g, ve).replace(/punctSpace/g, qe).replace(/punct/g, pe).getRegex(), je = d("^[^_*]*?\\*\\*[^_*]*?_[^_*]*?(?=\\*\\*)|[^_]+(?=[^_])|(?!_)punct(_+)(?=[\\s]|$)|notPunctSpace(_+)(?!_)(?=punctSpace|$)|(?!_)punctSpace(_+)(?=notPunctSpace)|[\\s](_+)(?!_)(?=punct)|(?!_)punct(_+)(?!_)(?=punct)", "gu").replace(/notPunctSpace/g, X).replace(/punctSpace/g, Z).replace(/punct/g, I).getRegex(), Fe = d(/^~~?(?:((?!~)punct)|[^\s~])/, "u").replace(/punct/g, I).getRegex(), Ue = "^[^~]+(?=[^~])|(?!~)punct(~~?)(?=[\\s]|$)|notPunctSpace(~~?)(?!~)(?=punctSpace|$)|(?!~)punctSpace(~~?)(?=notPunctSpace)|[\\s](~~?)(?!~)(?=punct)|(?!~)punct(~~?)(?!~)(?=punct)|notPunctSpace(~~?)(?=notPunctSpace)", Ke = d(Ue, "gu").replace(/notPunctSpace/g, X).replace(/punctSpace/g, Z).replace(/punct/g, I).getRegex(), We = d(/\\(punct)/, "gu").replace(/punct/g, I).getRegex(), Xe = d(/^<(scheme:[^\s\x00-\x1f<>]*|email)>/).replace("scheme", /[a-zA-Z][a-zA-Z0-9+.-]{1,31}/).replace("email", /[a-zA-Z0-9.!#$%&'*+/=?^_`{|}~-]+(@)[a-zA-Z0-9](?:[a-zA-Z0-9-]{0,61}[a-zA-Z0-9])?(?:\.[a-zA-Z0-9](?:[a-zA-Z0-9-]{0,61}[a-zA-Z0-9])?)+(?![-_])/).getRegex(), Je = d(K).replace("(?:-->|$)", "-->").getRegex(), Ve = d("^comment|^</[a-zA-Z][\\w:-]*\\s*>|^<[a-zA-Z][\\w-]*(?:attribute)*?\\s*/?>|^<\\?[\\s\\S]*?\\?>|^<![a-zA-Z]+\\s[\\s\\S]*?>|^<!\\[CDATA\\[[\\s\\S]*?\\]\\]>").replace("comment", Je).replace("attribute", /\s+[a-zA-Z:_][\w.:-]*(?:\s*=\s*"[^"]*"|\s*=\s*'[^']*'|\s*=\s*[^\s"'=<>`]+)?/).getRegex(), v = /(?:\[(?:\\[\s\S]|[^\[\]\\])*\]|\\[\s\S]|`+(?!`)[^`]*?`+(?!`)|``+(?=\])|[^\[\]\\`])*?/, Ye = d(/^!?\[(label)\]\(\s*(href)(?:(?:[ \t]+(?:\n[ \t]*)?|\n[ \t]*)(title))?\s*\)/).replace("label", v).replace("href", /<(?:\\.|[^\n<>\\])+>|[^ \t\n\x00-\x1f]*/).replace("title", /"(?:\\"?|[^"\\])*"|'(?:\\'?|[^'\\])*'|\((?:\\\)?|[^)\\])*\)/).getRegex(), ke = d(/^!?\[(label)\]\[(ref)\]/).replace("label", v).replace("ref", U).getRegex(), de = d(/^!?\[(ref)\](?:\[\])?/).replace("ref", U).getRegex(), et = d("reflink|nolink(?!\\()", "g").replace("reflink", ke).replace("nolink", de).getRegex(), ie = /[hH][tT][tT][pP][sS]?|[fF][tT][pP]/, J = { _backpedal: _, anyPunctuation: We, autolink: Xe, blockSkip: He, br: ue, code: Ce, del: _, delLDelim: _, delRDelim: _, emStrongLDelim: Ze, emStrongRDelimAst: Ne, emStrongRDelimUnd: je, escape: Ae, link: Ye, nolink: de, punctuation: De, reflink: ke, reflinkSearch: et, tag: Ve, text: Be, url: _ }, tt = { ...J, link: d(/^!?\[(label)\]\((.*?)\)/).replace("label", v).getRegex(), reflink: d(/^!?\[(label)\]\s*\[([^\]]*)\]/).replace("label", v).getRegex() }, Q = { ...J, emStrongRDelimAst: Qe, emStrongLDelim: Ge, delLDelim: Fe, delRDelim: Ke, url: d(/^((?:protocol):\/\/|www\.)(?:[a-zA-Z0-9\-]+\.?)+[^\s<]*|^email/).replace("protocol", ie).replace("email", /[A-Za-z0-9._+-]+(@)[a-zA-Z0-9-_]+(?:\.[a-zA-Z0-9-_]*[a-zA-Z0-9])+(?![-_])/).getRegex(), _backpedal: /(?:[^?!.,:;*_'"~()&]+|\([^)]*\)|&(?![a-zA-Z0-9]+;$)|[?!.,:;*_'"~)]+(?!$))+/, del: /^(~~?)(?=[^\s~])((?:\\[\s\S]|[^\\])*?(?:\\[\s\S]|[^\s~\\]))\1(?=[^~]|$)/, text: d(/^([`~]+|[^`~])(?:(?= {2,}\n)|(?=[a-zA-Z0-9.!#$%&'*+\/=?_`{\|}~-]+@)|[\s\S]*?(?:(?=[\\<!\[`*~_]|\b_|protocol:\/\/|www\.|$)|[^ ](?= {2,}\n)|[^a-zA-Z0-9.!#$%&'*+\/=?_`{\|}~-](?=[a-zA-Z0-9.!#$%&'*+\/=?_`{\|}~-]+@)))/).replace("protocol", ie).getRegex() }, nt = { ...Q, br: d(ue).replace("{2,}", "*").getRegex(), text: d(Q.text).replace("\\b_", "\\b_| {2,}\\n").replace(/\{2,\}/g, "*").getRegex() }, D = { normal: W, gfm: Ee, pedantic: Ie }, A = { normal: J, gfm: Q, breaks: nt, pedantic: tt };
var rt = { "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;", "'": "&#39;" }, ge = (l3) => rt[l3];
function O(l3, e) {
  if (e) {
    if (m.escapeTest.test(l3)) return l3.replace(m.escapeReplace, ge);
  } else if (m.escapeTestNoEncode.test(l3)) return l3.replace(m.escapeReplaceNoEncode, ge);
  return l3;
}
function V(l3) {
  try {
    l3 = encodeURI(l3).replace(m.percentDecode, "%");
  } catch {
    return null;
  }
  return l3;
}
function Y(l3, e) {
  let t = l3.replace(m.findPipe, (r, i, o) => {
    let u = false, a = i;
    for (; --a >= 0 && o[a] === "\\"; ) u = !u;
    return u ? "|" : " |";
  }), n = t.split(m.splitPipe), s = 0;
  if (n[0].trim() || n.shift(), n.length > 0 && !n.at(-1)?.trim() && n.pop(), e) if (n.length > e) n.splice(e);
  else for (; n.length < e; ) n.push("");
  for (; s < n.length; s++) n[s] = n[s].trim().replace(m.slashPipe, "|");
  return n;
}
function $(l3, e, t) {
  let n = l3.length;
  if (n === 0) return "";
  let s = 0;
  for (; s < n; ) {
    let r = l3.charAt(n - s - 1);
    if (r === e && true) s++;
    else break;
  }
  return l3.slice(0, n - s);
}
function ee(l3) {
  let e = l3.split(`
`), t = e.length - 1;
  for (; t >= 0 && m.blankLine.test(e[t]); ) t--;
  return e.length - t <= 2 ? l3 : e.slice(0, t + 1).join(`
`);
}
function fe(l3, e) {
  if (l3.indexOf(e[1]) === -1) return -1;
  let t = 0;
  for (let n = 0; n < l3.length; n++) if (l3[n] === "\\") n++;
  else if (l3[n] === e[0]) t++;
  else if (l3[n] === e[1] && (t--, t < 0)) return n;
  return t > 0 ? -2 : -1;
}
function me(l3, e = 0) {
  let t = e, n = "";
  for (let s of l3) if (s === "	") {
    let r = 4 - t % 4;
    n += " ".repeat(r), t += r;
  } else n += s, t++;
  return n;
}
function xe(l3, e, t, n, s) {
  let r = e.href, i = e.title || null, o = l3[1].replace(s.other.outputLinkReplace, "$1");
  n.state.inLink = true;
  let u = { type: l3[0].charAt(0) === "!" ? "image" : "link", raw: t, href: r, title: i, text: o, tokens: n.inlineTokens(o) };
  return n.state.inLink = false, u;
}
function st(l3, e, t) {
  let n = l3.match(t.other.indentCodeCompensation);
  if (n === null) return e;
  let s = n[1];
  return e.split(`
`).map((r) => {
    let i = r.match(t.other.beginningSpace);
    if (i === null) return r;
    let [o] = i;
    return o.length >= s.length ? r.slice(s.length) : r;
  }).join(`
`);
}
var w = class {
  constructor(e) {
    __publicField(this, "options");
    __publicField(this, "rules");
    __publicField(this, "lexer");
    this.options = e || T;
  }
  space(e) {
    let t = this.rules.block.newline.exec(e);
    if (t && t[0].length > 0) return { type: "space", raw: t[0] };
  }
  code(e) {
    let t = this.rules.block.code.exec(e);
    if (t) {
      let n = this.options.pedantic ? t[0] : ee(t[0]), s = n.replace(this.rules.other.codeRemoveIndent, "");
      return { type: "code", raw: n, codeBlockStyle: "indented", text: s };
    }
  }
  fences(e) {
    let t = this.rules.block.fences.exec(e);
    if (t) {
      let n = t[0], s = st(n, t[3] || "", this.rules);
      return { type: "code", raw: n, lang: t[2] ? t[2].trim().replace(this.rules.inline.anyPunctuation, "$1") : t[2], text: s };
    }
  }
  heading(e) {
    let t = this.rules.block.heading.exec(e);
    if (t) {
      let n = t[2].trim();
      if (this.rules.other.endingHash.test(n)) {
        let s = $(n, "#");
        (this.options.pedantic || !s || this.rules.other.endingSpaceChar.test(s)) && (n = s.trim());
      }
      return { type: "heading", raw: $(t[0], `
`), depth: t[1].length, text: n, tokens: this.lexer.inline(n) };
    }
  }
  hr(e) {
    let t = this.rules.block.hr.exec(e);
    if (t) return { type: "hr", raw: $(t[0], `
`) };
  }
  blockquote(e) {
    let t = this.rules.block.blockquote.exec(e);
    if (t) {
      let n = $(t[0], `
`).split(`
`), s = "", r = "", i = [];
      for (; n.length > 0; ) {
        let o = false, u = [], a;
        for (a = 0; a < n.length; a++) if (this.rules.other.blockquoteStart.test(n[a])) u.push(n[a]), o = true;
        else if (!o) u.push(n[a]);
        else break;
        n = n.slice(a);
        let c = u.join(`
`), p = c.replace(this.rules.other.blockquoteSetextReplace, `
    $1`).replace(this.rules.other.blockquoteSetextReplace2, "");
        s = s ? `${s}
${c}` : c, r = r ? `${r}
${p}` : p;
        let k = this.lexer.state.top;
        if (this.lexer.state.top = true, this.lexer.blockTokens(p, i, true), this.lexer.state.top = k, n.length === 0) break;
        let h = i.at(-1);
        if (h?.type === "code") break;
        if (h?.type === "blockquote") {
          let R = h, f = R.raw + `
` + n.join(`
`), S = this.blockquote(f);
          i[i.length - 1] = S, s = s.substring(0, s.length - R.raw.length) + S.raw, r = r.substring(0, r.length - R.text.length) + S.text;
          break;
        } else if (h?.type === "list") {
          let R = h, f = R.raw + `
` + n.join(`
`), S = this.list(f);
          i[i.length - 1] = S, s = s.substring(0, s.length - h.raw.length) + S.raw, r = r.substring(0, r.length - R.raw.length) + S.raw, n = f.substring(i.at(-1).raw.length).split(`
`);
          continue;
        }
      }
      return { type: "blockquote", raw: s, tokens: i, text: r };
    }
  }
  list(e) {
    let t = this.rules.block.list.exec(e);
    if (t) {
      let n = t[1].trim(), s = n.length > 1, r = { type: "list", raw: "", ordered: s, start: s ? +n.slice(0, -1) : "", loose: false, items: [] };
      n = s ? `\\d{1,9}\\${n.slice(-1)}` : `\\${n}`, this.options.pedantic && (n = s ? n : "[*+-]");
      let i = this.rules.other.listItemRegex(n), o = false;
      for (; e; ) {
        let a = false, c = "", p = "";
        if (!(t = i.exec(e)) || this.rules.block.hr.test(e)) break;
        c = t[0], e = e.substring(c.length);
        let k = me(t[2].split(`
`, 1)[0], t[1].length), h = e.split(`
`, 1)[0], R = !k.trim(), f = 0;
        if (this.options.pedantic ? (f = 2, p = k.trimStart()) : R ? f = t[1].length + 1 : (f = k.search(this.rules.other.nonSpaceChar), f = f > 4 ? 1 : f, p = k.slice(f), f += t[1].length), R && this.rules.other.blankLine.test(h) && (c += h + `
`, e = e.substring(h.length + 1), a = true), !a) {
          let S = this.rules.other.nextBulletRegex(f), te = this.rules.other.hrRegex(f), ne = this.rules.other.fencesBeginRegex(f), re = this.rules.other.headingBeginRegex(f), be = this.rules.other.htmlBeginRegex(f), Re = this.rules.other.blockquoteBeginRegex(f);
          for (; e; ) {
            let G = e.split(`
`, 1)[0], C;
            if (h = G, this.options.pedantic ? (h = h.replace(this.rules.other.listReplaceNesting, "  "), C = h) : C = h.replace(this.rules.other.tabCharGlobal, "    "), ne.test(h) || re.test(h) || be.test(h) || Re.test(h) || S.test(h) || te.test(h)) break;
            if (C.search(this.rules.other.nonSpaceChar) >= f || !h.trim()) p += `
` + C.slice(f);
            else {
              if (R || k.replace(this.rules.other.tabCharGlobal, "    ").search(this.rules.other.nonSpaceChar) >= 4 || ne.test(k) || re.test(k) || te.test(k)) break;
              p += `
` + h;
            }
            R = !h.trim(), c += G + `
`, e = e.substring(G.length + 1), k = C.slice(f);
          }
        }
        r.loose || (o ? r.loose = true : this.rules.other.doubleBlankLine.test(c) && (o = true)), r.items.push({ type: "list_item", raw: c, task: !!this.options.gfm && this.rules.other.listIsTask.test(p), loose: false, text: p, tokens: [] }), r.raw += c;
      }
      let u = r.items.at(-1);
      if (u) u.raw = u.raw.trimEnd(), u.text = u.text.trimEnd();
      else return;
      r.raw = r.raw.trimEnd();
      for (let a of r.items) {
        this.lexer.state.top = false, a.tokens = this.lexer.blockTokens(a.text, []);
        let c = a.tokens[0];
        if (a.task && (c?.type === "text" || c?.type === "paragraph")) {
          a.text = a.text.replace(this.rules.other.listReplaceTask, ""), c.raw = c.raw.replace(this.rules.other.listReplaceTask, ""), c.text = c.text.replace(this.rules.other.listReplaceTask, "");
          for (let k = this.lexer.inlineQueue.length - 1; k >= 0; k--) if (this.rules.other.listIsTask.test(this.lexer.inlineQueue[k].src)) {
            this.lexer.inlineQueue[k].src = this.lexer.inlineQueue[k].src.replace(this.rules.other.listReplaceTask, "");
            break;
          }
          let p = this.rules.other.listTaskCheckbox.exec(a.raw);
          if (p) {
            let k = { type: "checkbox", raw: p[0] + " ", checked: p[0] !== "[ ]" };
            a.checked = k.checked, r.loose ? a.tokens[0] && ["paragraph", "text"].includes(a.tokens[0].type) && "tokens" in a.tokens[0] && a.tokens[0].tokens ? (a.tokens[0].raw = k.raw + a.tokens[0].raw, a.tokens[0].text = k.raw + a.tokens[0].text, a.tokens[0].tokens.unshift(k)) : a.tokens.unshift({ type: "paragraph", raw: k.raw, text: k.raw, tokens: [k] }) : a.tokens.unshift(k);
          }
        } else a.task && (a.task = false);
        if (!r.loose) {
          let p = a.tokens.filter((h) => h.type === "space"), k = p.length > 0 && p.some((h) => this.rules.other.anyLine.test(h.raw));
          r.loose = k;
        }
      }
      if (r.loose) for (let a of r.items) {
        a.loose = true;
        for (let c of a.tokens) c.type === "text" && (c.type = "paragraph");
      }
      return r;
    }
  }
  html(e) {
    let t = this.rules.block.html.exec(e);
    if (t) {
      let n = ee(t[0]);
      return { type: "html", block: true, raw: n, pre: t[1] === "pre" || t[1] === "script" || t[1] === "style", text: n };
    }
  }
  def(e) {
    let t = this.rules.block.def.exec(e);
    if (t) {
      let n = t[1].toLowerCase().replace(this.rules.other.multipleSpaceGlobal, " "), s = t[2] ? t[2].replace(this.rules.other.hrefBrackets, "$1").replace(this.rules.inline.anyPunctuation, "$1") : "", r = t[3] ? t[3].substring(1, t[3].length - 1).replace(this.rules.inline.anyPunctuation, "$1") : t[3];
      return { type: "def", tag: n, raw: $(t[0], `
`), href: s, title: r };
    }
  }
  table(e) {
    let t = this.rules.block.table.exec(e);
    if (!t || !this.rules.other.tableDelimiter.test(t[2])) return;
    let n = Y(t[1]), s = t[2].replace(this.rules.other.tableAlignChars, "").split("|"), r = t[3]?.trim() ? t[3].replace(this.rules.other.tableRowBlankLine, "").split(`
`) : [], i = { type: "table", raw: $(t[0], `
`), header: [], align: [], rows: [] };
    if (n.length === s.length) {
      for (let o of s) this.rules.other.tableAlignRight.test(o) ? i.align.push("right") : this.rules.other.tableAlignCenter.test(o) ? i.align.push("center") : this.rules.other.tableAlignLeft.test(o) ? i.align.push("left") : i.align.push(null);
      for (let o = 0; o < n.length; o++) i.header.push({ text: n[o], tokens: this.lexer.inline(n[o]), header: true, align: i.align[o] });
      for (let o of r) i.rows.push(Y(o, i.header.length).map((u, a) => ({ text: u, tokens: this.lexer.inline(u), header: false, align: i.align[a] })));
      return i;
    }
  }
  lheading(e) {
    let t = this.rules.block.lheading.exec(e);
    if (t) {
      let n = t[1].trim();
      return { type: "heading", raw: $(t[0], `
`), depth: t[2].charAt(0) === "=" ? 1 : 2, text: n, tokens: this.lexer.inline(n) };
    }
  }
  paragraph(e) {
    let t = this.rules.block.paragraph.exec(e);
    if (t) {
      let n = t[1].charAt(t[1].length - 1) === `
` ? t[1].slice(0, -1) : t[1];
      return { type: "paragraph", raw: t[0], text: n, tokens: this.lexer.inline(n) };
    }
  }
  text(e) {
    let t = this.rules.block.text.exec(e);
    if (t) return { type: "text", raw: t[0], text: t[0], tokens: this.lexer.inline(t[0]) };
  }
  escape(e) {
    let t = this.rules.inline.escape.exec(e);
    if (t) return { type: "escape", raw: t[0], text: t[1] };
  }
  tag(e) {
    let t = this.rules.inline.tag.exec(e);
    if (t) return !this.lexer.state.inLink && this.rules.other.startATag.test(t[0]) ? this.lexer.state.inLink = true : this.lexer.state.inLink && this.rules.other.endATag.test(t[0]) && (this.lexer.state.inLink = false), !this.lexer.state.inRawBlock && this.rules.other.startPreScriptTag.test(t[0]) ? this.lexer.state.inRawBlock = true : this.lexer.state.inRawBlock && this.rules.other.endPreScriptTag.test(t[0]) && (this.lexer.state.inRawBlock = false), { type: "html", raw: t[0], inLink: this.lexer.state.inLink, inRawBlock: this.lexer.state.inRawBlock, block: false, text: t[0] };
  }
  link(e) {
    let t = this.rules.inline.link.exec(e);
    if (t) {
      let n = t[2].trim();
      if (!this.options.pedantic && this.rules.other.startAngleBracket.test(n)) {
        if (!this.rules.other.endAngleBracket.test(n)) return;
        let i = $(n.slice(0, -1), "\\");
        if ((n.length - i.length) % 2 === 0) return;
      } else {
        let i = fe(t[2], "()");
        if (i === -2) return;
        if (i > -1) {
          let u = (t[0].indexOf("!") === 0 ? 5 : 4) + t[1].length + i;
          t[2] = t[2].substring(0, i), t[0] = t[0].substring(0, u).trim(), t[3] = "";
        }
      }
      let s = t[2], r = "";
      if (this.options.pedantic) {
        let i = this.rules.other.pedanticHrefTitle.exec(s);
        i && (s = i[1], r = i[3]);
      } else r = t[3] ? t[3].slice(1, -1) : "";
      return s = s.trim(), this.rules.other.startAngleBracket.test(s) && (this.options.pedantic && !this.rules.other.endAngleBracket.test(n) ? s = s.slice(1) : s = s.slice(1, -1)), xe(t, { href: s && s.replace(this.rules.inline.anyPunctuation, "$1"), title: r && r.replace(this.rules.inline.anyPunctuation, "$1") }, t[0], this.lexer, this.rules);
    }
  }
  reflink(e, t) {
    let n;
    if ((n = this.rules.inline.reflink.exec(e)) || (n = this.rules.inline.nolink.exec(e))) {
      let s = (n[2] || n[1]).replace(this.rules.other.multipleSpaceGlobal, " "), r = t[s.toLowerCase()];
      if (!r) {
        let i = n[0].charAt(0);
        return { type: "text", raw: i, text: i };
      }
      return xe(n, r, n[0], this.lexer, this.rules);
    }
  }
  emStrong(e, t, n = "") {
    let s = this.rules.inline.emStrongLDelim.exec(e);
    if (!s || !s[1] && !s[2] && !s[3] && !s[4] || s[4] && n.match(this.rules.other.unicodeAlphaNumeric)) return;
    if (!(s[1] || s[3] || "") || !n || this.rules.inline.punctuation.exec(n)) {
      let i = [...s[0]].length - 1, o, u, a = i, c = 0, p = s[0][0] === "*" ? this.rules.inline.emStrongRDelimAst : this.rules.inline.emStrongRDelimUnd;
      for (p.lastIndex = 0, t = t.slice(-1 * e.length + i); (s = p.exec(t)) !== null; ) {
        if (o = s[1] || s[2] || s[3] || s[4] || s[5] || s[6], !o) continue;
        if (u = [...o].length, s[3] || s[4]) {
          a += u;
          continue;
        } else if ((s[5] || s[6]) && i % 3 && !((i + u) % 3)) {
          c += u;
          continue;
        }
        if (a -= u, a > 0) continue;
        u = Math.min(u, u + a + c);
        let k = [...s[0]][0].length, h = e.slice(0, i + s.index + k + u);
        if (Math.min(i, u) % 2) {
          let f = h.slice(1, -1);
          return { type: "em", raw: h, text: f, tokens: this.lexer.inlineTokens(f) };
        }
        let R = h.slice(2, -2);
        return { type: "strong", raw: h, text: R, tokens: this.lexer.inlineTokens(R) };
      }
    }
  }
  codespan(e) {
    let t = this.rules.inline.code.exec(e);
    if (t) {
      let n = t[2].replace(this.rules.other.newLineCharGlobal, " "), s = this.rules.other.nonSpaceChar.test(n), r = this.rules.other.startingSpaceChar.test(n) && this.rules.other.endingSpaceChar.test(n);
      return s && r && (n = n.substring(1, n.length - 1)), { type: "codespan", raw: t[0], text: n };
    }
  }
  br(e) {
    let t = this.rules.inline.br.exec(e);
    if (t) return { type: "br", raw: t[0] };
  }
  del(e, t, n = "") {
    let s = this.rules.inline.delLDelim.exec(e);
    if (!s) return;
    if (!(s[1] || "") || !n || this.rules.inline.punctuation.exec(n)) {
      let i = [...s[0]].length - 1, o, u, a = i, c = this.rules.inline.delRDelim;
      for (c.lastIndex = 0, t = t.slice(-1 * e.length + i); (s = c.exec(t)) !== null; ) {
        if (o = s[1] || s[2] || s[3] || s[4] || s[5] || s[6], !o || (u = [...o].length, u !== i)) continue;
        if (s[3] || s[4]) {
          a += u;
          continue;
        }
        if (a -= u, a > 0) continue;
        u = Math.min(u, u + a);
        let p = [...s[0]][0].length, k = e.slice(0, i + s.index + p + u), h = k.slice(i, -i);
        return { type: "del", raw: k, text: h, tokens: this.lexer.inlineTokens(h) };
      }
    }
  }
  autolink(e) {
    let t = this.rules.inline.autolink.exec(e);
    if (t) {
      let n, s;
      return t[2] === "@" ? (n = t[1], s = "mailto:" + n) : (n = t[1], s = n), { type: "link", raw: t[0], text: n, href: s, tokens: [{ type: "text", raw: n, text: n }] };
    }
  }
  url(e) {
    let t;
    if (t = this.rules.inline.url.exec(e)) {
      let n, s;
      if (t[2] === "@") n = t[0], s = "mailto:" + n;
      else {
        let r;
        do
          r = t[0], t[0] = this.rules.inline._backpedal.exec(t[0])?.[0] ?? "";
        while (r !== t[0]);
        n = t[0], t[1] === "www." ? s = "http://" + t[0] : s = t[0];
      }
      return { type: "link", raw: t[0], text: n, href: s, tokens: [{ type: "text", raw: n, text: n }] };
    }
  }
  inlineText(e) {
    let t = this.rules.inline.text.exec(e);
    if (t) {
      let n = this.lexer.state.inRawBlock;
      return { type: "text", raw: t[0], text: t[0], escaped: n };
    }
  }
};
var x = class l {
  constructor(e) {
    __publicField(this, "tokens");
    __publicField(this, "options");
    __publicField(this, "state");
    __publicField(this, "inlineQueue");
    __publicField(this, "tokenizer");
    this.tokens = [], this.tokens.links = /* @__PURE__ */ Object.create(null), this.options = e || T, this.options.tokenizer = this.options.tokenizer || new w(), this.tokenizer = this.options.tokenizer, this.tokenizer.options = this.options, this.tokenizer.lexer = this, this.inlineQueue = [], this.state = { inLink: false, inRawBlock: false, top: true };
    let t = { other: m, block: D.normal, inline: A.normal };
    this.options.pedantic ? (t.block = D.pedantic, t.inline = A.pedantic) : this.options.gfm && (t.block = D.gfm, this.options.breaks ? t.inline = A.breaks : t.inline = A.gfm), this.tokenizer.rules = t;
  }
  static get rules() {
    return { block: D, inline: A };
  }
  static lex(e, t) {
    return new l(t).lex(e);
  }
  static lexInline(e, t) {
    return new l(t).inlineTokens(e);
  }
  lex(e) {
    e = e.replace(m.carriageReturn, `
`), this.blockTokens(e, this.tokens);
    for (let t = 0; t < this.inlineQueue.length; t++) {
      let n = this.inlineQueue[t];
      this.inlineTokens(n.src, n.tokens);
    }
    return this.inlineQueue = [], this.tokens;
  }
  blockTokens(e, t = [], n = false) {
    this.tokenizer.lexer = this, this.options.pedantic && (e = e.replace(m.tabCharGlobal, "    ").replace(m.spaceLine, ""));
    let s = 1 / 0;
    for (; e; ) {
      if (e.length < s) s = e.length;
      else {
        this.infiniteLoopError(e.charCodeAt(0));
        break;
      }
      let r;
      if (this.options.extensions?.block?.some((o) => (r = o.call({ lexer: this }, e, t)) ? (e = e.substring(r.raw.length), t.push(r), true) : false)) continue;
      if (r = this.tokenizer.space(e)) {
        e = e.substring(r.raw.length);
        let o = t.at(-1);
        r.raw.length === 1 && o !== void 0 ? o.raw += `
` : t.push(r);
        continue;
      }
      if (r = this.tokenizer.code(e)) {
        e = e.substring(r.raw.length);
        let o = t.at(-1);
        o?.type === "paragraph" || o?.type === "text" ? (o.raw += (o.raw.endsWith(`
`) ? "" : `
`) + r.raw, o.text += `
` + r.text, this.inlineQueue.at(-1).src = o.text) : t.push(r);
        continue;
      }
      if (r = this.tokenizer.fences(e)) {
        e = e.substring(r.raw.length), t.push(r);
        continue;
      }
      if (r = this.tokenizer.heading(e)) {
        e = e.substring(r.raw.length), t.push(r);
        continue;
      }
      if (r = this.tokenizer.hr(e)) {
        e = e.substring(r.raw.length), t.push(r);
        continue;
      }
      if (r = this.tokenizer.blockquote(e)) {
        e = e.substring(r.raw.length), t.push(r);
        continue;
      }
      if (r = this.tokenizer.list(e)) {
        e = e.substring(r.raw.length), t.push(r);
        continue;
      }
      if (r = this.tokenizer.html(e)) {
        e = e.substring(r.raw.length), t.push(r);
        continue;
      }
      if (r = this.tokenizer.def(e)) {
        e = e.substring(r.raw.length);
        let o = t.at(-1);
        o?.type === "paragraph" || o?.type === "text" ? (o.raw += (o.raw.endsWith(`
`) ? "" : `
`) + r.raw, o.text += `
` + r.raw, this.inlineQueue.at(-1).src = o.text) : this.tokens.links[r.tag] || (this.tokens.links[r.tag] = { href: r.href, title: r.title }, t.push(r));
        continue;
      }
      if (r = this.tokenizer.table(e)) {
        e = e.substring(r.raw.length), t.push(r);
        continue;
      }
      if (r = this.tokenizer.lheading(e)) {
        e = e.substring(r.raw.length), t.push(r);
        continue;
      }
      let i = e;
      if (this.options.extensions?.startBlock) {
        let o = 1 / 0, u = e.slice(1), a;
        this.options.extensions.startBlock.forEach((c) => {
          a = c.call({ lexer: this }, u), typeof a == "number" && a >= 0 && (o = Math.min(o, a));
        }), o < 1 / 0 && o >= 0 && (i = e.substring(0, o + 1));
      }
      if (this.state.top && (r = this.tokenizer.paragraph(i))) {
        let o = t.at(-1);
        n && o?.type === "paragraph" ? (o.raw += (o.raw.endsWith(`
`) ? "" : `
`) + r.raw, o.text += `
` + r.text, this.inlineQueue.pop(), this.inlineQueue.at(-1).src = o.text) : t.push(r), n = i.length !== e.length, e = e.substring(r.raw.length);
        continue;
      }
      if (r = this.tokenizer.text(e)) {
        e = e.substring(r.raw.length);
        let o = t.at(-1);
        o?.type === "text" ? (o.raw += (o.raw.endsWith(`
`) ? "" : `
`) + r.raw, o.text += `
` + r.text, this.inlineQueue.pop(), this.inlineQueue.at(-1).src = o.text) : t.push(r);
        continue;
      }
      if (e) {
        this.infiniteLoopError(e.charCodeAt(0));
        break;
      }
    }
    return this.state.top = true, t;
  }
  inline(e, t = []) {
    return this.inlineQueue.push({ src: e, tokens: t }), t;
  }
  inlineTokens(e, t = []) {
    this.tokenizer.lexer = this;
    let n = e, s = null;
    if (this.tokens.links) {
      let a = Object.keys(this.tokens.links);
      if (a.length > 0) for (; (s = this.tokenizer.rules.inline.reflinkSearch.exec(n)) !== null; ) a.includes(s[0].slice(s[0].lastIndexOf("[") + 1, -1)) && (n = n.slice(0, s.index) + "[" + "a".repeat(s[0].length - 2) + "]" + n.slice(this.tokenizer.rules.inline.reflinkSearch.lastIndex));
    }
    for (; (s = this.tokenizer.rules.inline.anyPunctuation.exec(n)) !== null; ) n = n.slice(0, s.index) + "++" + n.slice(this.tokenizer.rules.inline.anyPunctuation.lastIndex);
    let r;
    for (; (s = this.tokenizer.rules.inline.blockSkip.exec(n)) !== null; ) r = s[2] ? s[2].length : 0, n = n.slice(0, s.index + r) + "[" + "a".repeat(s[0].length - r - 2) + "]" + n.slice(this.tokenizer.rules.inline.blockSkip.lastIndex);
    n = this.options.hooks?.emStrongMask?.call({ lexer: this }, n) ?? n;
    let i = false, o = "", u = 1 / 0;
    for (; e; ) {
      if (e.length < u) u = e.length;
      else {
        this.infiniteLoopError(e.charCodeAt(0));
        break;
      }
      i || (o = ""), i = false;
      let a;
      if (this.options.extensions?.inline?.some((p) => (a = p.call({ lexer: this }, e, t)) ? (e = e.substring(a.raw.length), t.push(a), true) : false)) continue;
      if (a = this.tokenizer.escape(e)) {
        e = e.substring(a.raw.length), t.push(a);
        continue;
      }
      if (a = this.tokenizer.tag(e)) {
        e = e.substring(a.raw.length), t.push(a);
        continue;
      }
      if (a = this.tokenizer.link(e)) {
        e = e.substring(a.raw.length), t.push(a);
        continue;
      }
      if (a = this.tokenizer.reflink(e, this.tokens.links)) {
        e = e.substring(a.raw.length);
        let p = t.at(-1);
        a.type === "text" && p?.type === "text" ? (p.raw += a.raw, p.text += a.text) : t.push(a);
        continue;
      }
      if (a = this.tokenizer.emStrong(e, n, o)) {
        e = e.substring(a.raw.length), t.push(a);
        continue;
      }
      if (a = this.tokenizer.codespan(e)) {
        e = e.substring(a.raw.length), t.push(a);
        continue;
      }
      if (a = this.tokenizer.br(e)) {
        e = e.substring(a.raw.length), t.push(a);
        continue;
      }
      if (a = this.tokenizer.del(e, n, o)) {
        e = e.substring(a.raw.length), t.push(a);
        continue;
      }
      if (a = this.tokenizer.autolink(e)) {
        e = e.substring(a.raw.length), t.push(a);
        continue;
      }
      if (!this.state.inLink && (a = this.tokenizer.url(e))) {
        e = e.substring(a.raw.length), t.push(a);
        continue;
      }
      let c = e;
      if (this.options.extensions?.startInline) {
        let p = 1 / 0, k = e.slice(1), h;
        this.options.extensions.startInline.forEach((R) => {
          h = R.call({ lexer: this }, k), typeof h == "number" && h >= 0 && (p = Math.min(p, h));
        }), p < 1 / 0 && p >= 0 && (c = e.substring(0, p + 1));
      }
      if (a = this.tokenizer.inlineText(c)) {
        e = e.substring(a.raw.length), a.raw.slice(-1) !== "_" && (o = a.raw.slice(-1)), i = true;
        let p = t.at(-1);
        p?.type === "text" ? (p.raw += a.raw, p.text += a.text) : t.push(a);
        continue;
      }
      if (e) {
        this.infiniteLoopError(e.charCodeAt(0));
        break;
      }
    }
    return t;
  }
  infiniteLoopError(e) {
    let t = "Infinite loop on byte: " + e;
    if (this.options.silent) console.error(t);
    else throw new Error(t);
  }
};
var y = class {
  constructor(e) {
    __publicField(this, "options");
    __publicField(this, "parser");
    this.options = e || T;
  }
  space(e) {
    return "";
  }
  code({ text: e, lang: t, escaped: n }) {
    let s = (t || "").match(m.notSpaceStart)?.[0], r = e.replace(m.endingNewline, "") + `
`;
    return s ? '<pre><code class="language-' + O(s) + '">' + (n ? r : O(r, true)) + `</code></pre>
` : "<pre><code>" + (n ? r : O(r, true)) + `</code></pre>
`;
  }
  blockquote({ tokens: e }) {
    return `<blockquote>
${this.parser.parse(e)}</blockquote>
`;
  }
  html({ text: e }) {
    return e;
  }
  def(e) {
    return "";
  }
  heading({ tokens: e, depth: t }) {
    return `<h${t}>${this.parser.parseInline(e)}</h${t}>
`;
  }
  hr(e) {
    return `<hr>
`;
  }
  list(e) {
    let t = e.ordered, n = e.start, s = "";
    for (let o = 0; o < e.items.length; o++) {
      let u = e.items[o];
      s += this.listitem(u);
    }
    let r = t ? "ol" : "ul", i = t && n !== 1 ? ' start="' + n + '"' : "";
    return "<" + r + i + `>
` + s + "</" + r + `>
`;
  }
  listitem(e) {
    return `<li>${this.parser.parse(e.tokens)}</li>
`;
  }
  checkbox({ checked: e }) {
    return "<input " + (e ? 'checked="" ' : "") + 'disabled="" type="checkbox"> ';
  }
  paragraph({ tokens: e }) {
    return `<p>${this.parser.parseInline(e)}</p>
`;
  }
  table(e) {
    let t = "", n = "";
    for (let r = 0; r < e.header.length; r++) n += this.tablecell(e.header[r]);
    t += this.tablerow({ text: n });
    let s = "";
    for (let r = 0; r < e.rows.length; r++) {
      let i = e.rows[r];
      n = "";
      for (let o = 0; o < i.length; o++) n += this.tablecell(i[o]);
      s += this.tablerow({ text: n });
    }
    return s && (s = `<tbody>${s}</tbody>`), `<table>
<thead>
` + t + `</thead>
` + s + `</table>
`;
  }
  tablerow({ text: e }) {
    return `<tr>
${e}</tr>
`;
  }
  tablecell(e) {
    let t = this.parser.parseInline(e.tokens), n = e.header ? "th" : "td";
    return (e.align ? `<${n} align="${e.align}">` : `<${n}>`) + t + `</${n}>
`;
  }
  strong({ tokens: e }) {
    return `<strong>${this.parser.parseInline(e)}</strong>`;
  }
  em({ tokens: e }) {
    return `<em>${this.parser.parseInline(e)}</em>`;
  }
  codespan({ text: e }) {
    return `<code>${O(e, true)}</code>`;
  }
  br(e) {
    return "<br>";
  }
  del({ tokens: e }) {
    return `<del>${this.parser.parseInline(e)}</del>`;
  }
  link({ href: e, title: t, tokens: n }) {
    let s = this.parser.parseInline(n), r = V(e);
    if (r === null) return s;
    e = r;
    let i = '<a href="' + e + '"';
    return t && (i += ' title="' + O(t) + '"'), i += ">" + s + "</a>", i;
  }
  image({ href: e, title: t, text: n, tokens: s }) {
    s && (n = this.parser.parseInline(s, this.parser.textRenderer));
    let r = V(e);
    if (r === null) return O(n);
    e = r;
    let i = `<img src="${e}" alt="${O(n)}"`;
    return t && (i += ` title="${O(t)}"`), i += ">", i;
  }
  text(e) {
    return "tokens" in e && e.tokens ? this.parser.parseInline(e.tokens) : "escaped" in e && e.escaped ? e.text : O(e.text);
  }
};
var L = class {
  strong({ text: e }) {
    return e;
  }
  em({ text: e }) {
    return e;
  }
  codespan({ text: e }) {
    return e;
  }
  del({ text: e }) {
    return e;
  }
  html({ text: e }) {
    return e;
  }
  text({ text: e }) {
    return e;
  }
  link({ text: e }) {
    return "" + e;
  }
  image({ text: e }) {
    return "" + e;
  }
  br() {
    return "";
  }
  checkbox({ raw: e }) {
    return e;
  }
};
var b = class l2 {
  constructor(e) {
    __publicField(this, "options");
    __publicField(this, "renderer");
    __publicField(this, "textRenderer");
    this.options = e || T, this.options.renderer = this.options.renderer || new y(), this.renderer = this.options.renderer, this.renderer.options = this.options, this.renderer.parser = this, this.textRenderer = new L();
  }
  static parse(e, t) {
    return new l2(t).parse(e);
  }
  static parseInline(e, t) {
    return new l2(t).parseInline(e);
  }
  parse(e) {
    this.renderer.parser = this;
    let t = "";
    for (let n = 0; n < e.length; n++) {
      let s = e[n];
      if (this.options.extensions?.renderers?.[s.type]) {
        let i = s, o = this.options.extensions.renderers[i.type].call({ parser: this }, i);
        if (o !== false || !["space", "hr", "heading", "code", "table", "blockquote", "list", "html", "def", "paragraph", "text"].includes(i.type)) {
          t += o || "";
          continue;
        }
      }
      let r = s;
      switch (r.type) {
        case "space": {
          t += this.renderer.space(r);
          break;
        }
        case "hr": {
          t += this.renderer.hr(r);
          break;
        }
        case "heading": {
          t += this.renderer.heading(r);
          break;
        }
        case "code": {
          t += this.renderer.code(r);
          break;
        }
        case "table": {
          t += this.renderer.table(r);
          break;
        }
        case "blockquote": {
          t += this.renderer.blockquote(r);
          break;
        }
        case "list": {
          t += this.renderer.list(r);
          break;
        }
        case "checkbox": {
          t += this.renderer.checkbox(r);
          break;
        }
        case "html": {
          t += this.renderer.html(r);
          break;
        }
        case "def": {
          t += this.renderer.def(r);
          break;
        }
        case "paragraph": {
          t += this.renderer.paragraph(r);
          break;
        }
        case "text": {
          t += this.renderer.text(r);
          break;
        }
        default: {
          let i = 'Token with "' + r.type + '" type was not found.';
          if (this.options.silent) return console.error(i), "";
          throw new Error(i);
        }
      }
    }
    return t;
  }
  parseInline(e, t = this.renderer) {
    this.renderer.parser = this;
    let n = "";
    for (let s = 0; s < e.length; s++) {
      let r = e[s];
      if (this.options.extensions?.renderers?.[r.type]) {
        let o = this.options.extensions.renderers[r.type].call({ parser: this }, r);
        if (o !== false || !["escape", "html", "link", "image", "strong", "em", "codespan", "br", "del", "text"].includes(r.type)) {
          n += o || "";
          continue;
        }
      }
      let i = r;
      switch (i.type) {
        case "escape": {
          n += t.text(i);
          break;
        }
        case "html": {
          n += t.html(i);
          break;
        }
        case "link": {
          n += t.link(i);
          break;
        }
        case "image": {
          n += t.image(i);
          break;
        }
        case "checkbox": {
          n += t.checkbox(i);
          break;
        }
        case "strong": {
          n += t.strong(i);
          break;
        }
        case "em": {
          n += t.em(i);
          break;
        }
        case "codespan": {
          n += t.codespan(i);
          break;
        }
        case "br": {
          n += t.br(i);
          break;
        }
        case "del": {
          n += t.del(i);
          break;
        }
        case "text": {
          n += t.text(i);
          break;
        }
        default: {
          let o = 'Token with "' + i.type + '" type was not found.';
          if (this.options.silent) return console.error(o), "";
          throw new Error(o);
        }
      }
    }
    return n;
  }
};
var P = (_a = class {
  constructor(e) {
    __publicField(this, "options");
    __publicField(this, "block");
    this.options = e || T;
  }
  preprocess(e) {
    return e;
  }
  postprocess(e) {
    return e;
  }
  processAllTokens(e) {
    return e;
  }
  emStrongMask(e) {
    return e;
  }
  provideLexer(e = this.block) {
    return e ? x.lex : x.lexInline;
  }
  provideParser(e = this.block) {
    return e ? b.parse : b.parseInline;
  }
}, __publicField(_a, "passThroughHooks", /* @__PURE__ */ new Set(["preprocess", "postprocess", "processAllTokens", "emStrongMask"])), __publicField(_a, "passThroughHooksRespectAsync", /* @__PURE__ */ new Set(["preprocess", "postprocess", "processAllTokens"])), _a);
var q = class {
  constructor(...e) {
    __publicField(this, "defaults", M());
    __publicField(this, "options", this.setOptions);
    __publicField(this, "parse", this.parseMarkdown(true));
    __publicField(this, "parseInline", this.parseMarkdown(false));
    __publicField(this, "Parser", b);
    __publicField(this, "Renderer", y);
    __publicField(this, "TextRenderer", L);
    __publicField(this, "Lexer", x);
    __publicField(this, "Tokenizer", w);
    __publicField(this, "Hooks", P);
    this.use(...e);
  }
  walkTokens(e, t) {
    let n = [];
    for (let s of e) switch (n = n.concat(t.call(this, s)), s.type) {
      case "table": {
        let r = s;
        for (let i of r.header) n = n.concat(this.walkTokens(i.tokens, t));
        for (let i of r.rows) for (let o of i) n = n.concat(this.walkTokens(o.tokens, t));
        break;
      }
      case "list": {
        let r = s;
        n = n.concat(this.walkTokens(r.items, t));
        break;
      }
      default: {
        let r = s;
        this.defaults.extensions?.childTokens?.[r.type] ? this.defaults.extensions.childTokens[r.type].forEach((i) => {
          let o = r[i].flat(1 / 0);
          n = n.concat(this.walkTokens(o, t));
        }) : r.tokens && (n = n.concat(this.walkTokens(r.tokens, t)));
      }
    }
    return n;
  }
  use(...e) {
    let t = this.defaults.extensions || { renderers: {}, childTokens: {} };
    return e.forEach((n) => {
      let s = { ...n };
      if (s.async = this.defaults.async || s.async || false, n.extensions && (n.extensions.forEach((r) => {
        if (!r.name) throw new Error("extension name required");
        if ("renderer" in r) {
          let i = t.renderers[r.name];
          i ? t.renderers[r.name] = function(...o) {
            let u = r.renderer.apply(this, o);
            return u === false && (u = i.apply(this, o)), u;
          } : t.renderers[r.name] = r.renderer;
        }
        if ("tokenizer" in r) {
          if (!r.level || r.level !== "block" && r.level !== "inline") throw new Error("extension level must be 'block' or 'inline'");
          let i = t[r.level];
          i ? i.unshift(r.tokenizer) : t[r.level] = [r.tokenizer], r.start && (r.level === "block" ? t.startBlock ? t.startBlock.push(r.start) : t.startBlock = [r.start] : r.level === "inline" && (t.startInline ? t.startInline.push(r.start) : t.startInline = [r.start]));
        }
        "childTokens" in r && r.childTokens && (t.childTokens[r.name] = r.childTokens);
      }), s.extensions = t), n.renderer) {
        let r = this.defaults.renderer || new y(this.defaults);
        for (let i in n.renderer) {
          if (!(i in r)) throw new Error(`renderer '${i}' does not exist`);
          if (["options", "parser"].includes(i)) continue;
          let o = i, u = n.renderer[o], a = r[o];
          r[o] = (...c) => {
            let p = u.apply(r, c);
            return p === false && (p = a.apply(r, c)), p || "";
          };
        }
        s.renderer = r;
      }
      if (n.tokenizer) {
        let r = this.defaults.tokenizer || new w(this.defaults);
        for (let i in n.tokenizer) {
          if (!(i in r)) throw new Error(`tokenizer '${i}' does not exist`);
          if (["options", "rules", "lexer"].includes(i)) continue;
          let o = i, u = n.tokenizer[o], a = r[o];
          r[o] = (...c) => {
            let p = u.apply(r, c);
            return p === false && (p = a.apply(r, c)), p;
          };
        }
        s.tokenizer = r;
      }
      if (n.hooks) {
        let r = this.defaults.hooks || new P();
        for (let i in n.hooks) {
          if (!(i in r)) throw new Error(`hook '${i}' does not exist`);
          if (["options", "block"].includes(i)) continue;
          let o = i, u = n.hooks[o], a = r[o];
          P.passThroughHooks.has(i) ? r[o] = (c) => {
            if (this.defaults.async && P.passThroughHooksRespectAsync.has(i)) return (async () => {
              let k = await u.call(r, c);
              return a.call(r, k);
            })();
            let p = u.call(r, c);
            return a.call(r, p);
          } : r[o] = (...c) => {
            if (this.defaults.async) return (async () => {
              let k = await u.apply(r, c);
              return k === false && (k = await a.apply(r, c)), k;
            })();
            let p = u.apply(r, c);
            return p === false && (p = a.apply(r, c)), p;
          };
        }
        s.hooks = r;
      }
      if (n.walkTokens) {
        let r = this.defaults.walkTokens, i = n.walkTokens;
        s.walkTokens = function(o) {
          let u = [];
          return u.push(i.call(this, o)), r && (u = u.concat(r.call(this, o))), u;
        };
      }
      this.defaults = { ...this.defaults, ...s };
    }), this;
  }
  setOptions(e) {
    return this.defaults = { ...this.defaults, ...e }, this;
  }
  lexer(e, t) {
    return x.lex(e, t ?? this.defaults);
  }
  parser(e, t) {
    return b.parse(e, t ?? this.defaults);
  }
  parseMarkdown(e) {
    return (n, s) => {
      let r = { ...s }, i = { ...this.defaults, ...r }, o = this.onError(!!i.silent, !!i.async);
      if (this.defaults.async === true && r.async === false) return o(new Error("marked(): The async option was set to true by an extension. Remove async: false from the parse options object to return a Promise."));
      if (typeof n > "u" || n === null) return o(new Error("marked(): input parameter is undefined or null"));
      if (typeof n != "string") return o(new Error("marked(): input parameter is of type " + Object.prototype.toString.call(n) + ", string expected"));
      if (i.hooks && (i.hooks.options = i, i.hooks.block = e), i.async) return (async () => {
        let u = i.hooks ? await i.hooks.preprocess(n) : n, c = await (i.hooks ? await i.hooks.provideLexer(e) : e ? x.lex : x.lexInline)(u, i), p = i.hooks ? await i.hooks.processAllTokens(c) : c;
        i.walkTokens && await Promise.all(this.walkTokens(p, i.walkTokens));
        let h = await (i.hooks ? await i.hooks.provideParser(e) : e ? b.parse : b.parseInline)(p, i);
        return i.hooks ? await i.hooks.postprocess(h) : h;
      })().catch(o);
      try {
        i.hooks && (n = i.hooks.preprocess(n));
        let a = (i.hooks ? i.hooks.provideLexer(e) : e ? x.lex : x.lexInline)(n, i);
        i.hooks && (a = i.hooks.processAllTokens(a)), i.walkTokens && this.walkTokens(a, i.walkTokens);
        let p = (i.hooks ? i.hooks.provideParser(e) : e ? b.parse : b.parseInline)(a, i);
        return i.hooks && (p = i.hooks.postprocess(p)), p;
      } catch (u) {
        return o(u);
      }
    };
  }
  onError(e, t) {
    return (n) => {
      if (n.message += `
Please report this to https://github.com/markedjs/marked.`, e) {
        let s = "<p>An error occurred:</p><pre>" + O(n.message + "", true) + "</pre>";
        return t ? Promise.resolve(s) : s;
      }
      if (t) return Promise.reject(n);
      throw n;
    };
  }
};
var z = new q();
function g(l3, e) {
  return z.parse(l3, e);
}
g.options = g.setOptions = function(l3) {
  return z.setOptions(l3), g.defaults = z.defaults, N(g.defaults), g;
};
g.getDefaults = M;
g.defaults = T;
g.use = function(...l3) {
  return z.use(...l3), g.defaults = z.defaults, N(g.defaults), g;
};
g.walkTokens = function(l3, e) {
  return z.walkTokens(l3, e);
};
g.parseInline = z.parseInline;
g.Parser = b;
g.parser = b.parse;
g.Renderer = y;
g.TextRenderer = L;
g.Lexer = x;
g.lexer = x.lex;
g.Tokenizer = w;
g.Hooks = P;
g.parse = g;
g.options;
g.setOptions;
g.use;
g.walkTokens;
g.parseInline;
b.parse;
x.lex;
g.setOptions({
  gfm: true,
  breaks: true
});
const UUID_RE = /^[0-9a-fA-F]{8}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{12}$/;
function expandLinks(src, titles) {
  return src.replace(/\[\[([^\]]+)\]\]/g, (whole, inner) => {
    const trimmed = inner.trim();
    if (!UUID_RE.test(trimmed)) return whole;
    const title = titles[trimmed];
    if (title) {
      const safeTitle = escapeAttr(title);
      return `<a class="keepsake-link" href="/r/${trimmed}" data-uuid="${trimmed}">[[${safeTitle}]]</a>`;
    }
    return `<a class="keepsake-link keepsake-link--missing" href="/r/${trimmed}" data-uuid="${trimmed}">[[${trimmed} ??]]</a>`;
  });
}
function escapeAttr(s) {
  return s.replace(/&/g, "&amp;").replace(/"/g, "&quot;").replace(/</g, "&lt;").replace(/>/g, "&gt;");
}
function renderMarkdown(src, titles = {}) {
  if (!src) return "";
  const expanded = expandLinks(src, titles);
  const html2 = g.parse(expanded, { async: false });
  return purify.sanitize(html2, {
    USE_PROFILES: { html: true },
    ADD_ATTR: ["data-uuid", "class"]
  });
}
var _tmpl$$6 = /* @__PURE__ */ template(`<div class=markdown>`);
function Markdown(props) {
  const html2 = createMemo(() => renderMarkdown(props.source, props.titles ?? {}));
  return (() => {
    var _el$ = _tmpl$$6();
    createRenderEffect(() => _el$.innerHTML = html2());
    return _el$;
  })();
}
var _tmpl$$5 = /* @__PURE__ */ template(`<span class=title-emoji>`), _tmpl$2$5 = /* @__PURE__ */ template(`<dl class=detail-fields>`), _tmpl$3$5 = /* @__PURE__ */ template(`<details class=raw-details><summary>Raw JSON</summary><pre class=json-view>`), _tmpl$4$5 = /* @__PURE__ */ template(`<div class=page><header class=page-header><div><h1 class=page-title></h1><p class=page-sub><code class=id-mono></code></p></div><div class=page-actions><button class=btn></button><button class="btn btn-danger">🗑 Delete`), _tmpl$5$5 = /* @__PURE__ */ template(`<p class=muted>Loading…`), _tmpl$6$3 = /* @__PURE__ */ template(`<p class=muted>This record has no fields yet.`), _tmpl$7$3 = /* @__PURE__ */ template(`<dt class=detail-label>`), _tmpl$8$2 = /* @__PURE__ */ template(`<dd>`), _tmpl$9$1 = /* @__PURE__ */ template(`<span>`), _tmpl$0$1 = /* @__PURE__ */ template(`<section class=runbook-section><h3>Steps</h3><ol class=runbook-steps>`), _tmpl$1$1 = /* @__PURE__ */ template(`<div class=step-status>`), _tmpl$10$1 = /* @__PURE__ */ template(`<li><div class=step-title>`);
function asSteps(v2) {
  if (!Array.isArray(v2)) return null;
  return v2.filter((s) => typeof s === "object" && s !== null && typeof s.title === "string" && typeof s.body === "string");
}
function maskIfSensitive(fieldName, schema, value, reveal) {
  if (value == null || value === "") {
    return {
      display: "—",
      hidden: false
    };
  }
  if (!isSensitive(fieldName, schema) || reveal) {
    if (Array.isArray(value)) {
      return {
        display: value.join(", "),
        hidden: false
      };
    }
    if (typeof value === "object") {
      return {
        display: JSON.stringify(value, null, 2),
        hidden: false
      };
    }
    return {
      display: String(value),
      hidden: false
    };
  }
  if (Array.isArray(value)) {
    return {
      display: "•••",
      hidden: true
    };
  }
  if (typeof value === "object") {
    return {
      display: "•••",
      hidden: true
    };
  }
  const s = String(value);
  if (s.length <= 4) {
    return {
      display: "•".repeat(s.length),
      hidden: true
    };
  }
  return {
    display: "•".repeat(s.length - 4) + s.slice(-4),
    hidden: true
  };
}
function RecordDetail() {
  const params = useParams();
  const [reveal, setReveal] = createSignal(false);
  const [data, {
    refetch
  }] = createResource(() => ({
    id: params.id,
    reveal: reveal()
  }), (k) => api.showRecord(k.id, k.reveal));
  const [titles] = createResource(() => params.id, () => api.recordTitles().then((rows) => {
    const m2 = {};
    for (const r of rows) m2[r.id] = r.title;
    return m2;
  }));
  async function del() {
    if (!confirm("Delete this record? This cannot be undone.")) return;
    try {
      await api.deleteRecord(params.id);
      showToast("ok", "Record deleted");
      history.back();
    } catch (e) {
      showToast("err", String(e));
    }
  }
  const recordType = () => {
    const d2 = data();
    return d2?.type ?? null;
  };
  const fieldRows = () => {
    const d2 = data();
    if (!d2 || !d2.type) return [];
    const schema = SCHEMAS[d2.type] ?? [];
    const out = [];
    for (const f of schema) {
      if (f.name === "type" || f.name === "id" || f.name === "created_at" || f.name === "updated_at" || f.name === "created_by" || f.name === "updated_by" || f.name === "schema_version" || f.name === "steps") {
        continue;
      }
      if (!(f.name in d2)) continue;
      const raw = d2[f.name];
      if (raw == null || raw === "" || Array.isArray(raw) && raw.length === 0) {
        continue;
      }
      const {
        display,
        hidden
      } = maskIfSensitive(f.name, f, raw, reveal());
      const multiline = f.multiline === true || f.name === "body" || f.name === "notes" || f.name === "description" || f.name === "billing_address" || f.name === "address" || f.name === "details";
      out.push({
        name: f.name,
        label: f.label,
        value: display,
        hidden,
        multiline
      });
    }
    return out;
  };
  return (() => {
    var _el$ = _tmpl$4$5(), _el$2 = _el$.firstChild, _el$3 = _el$2.firstChild, _el$4 = _el$3.firstChild, _el$6 = _el$4.nextSibling, _el$7 = _el$6.firstChild, _el$8 = _el$3.nextSibling, _el$9 = _el$8.firstChild, _el$0 = _el$9.nextSibling;
    insert(_el$4, createComponent(Show, {
      get when() {
        return memo(() => !!recordType())() && META_BY_TYPE[recordType()];
      },
      get children() {
        return [(() => {
          var _el$5 = _tmpl$$5();
          insert(_el$5, () => META_BY_TYPE[recordType()].icon);
          return _el$5;
        })(), memo(() => META_BY_TYPE[recordType()].label)];
      }
    }));
    insert(_el$7, () => params.id);
    _el$9.$$click = () => {
      setReveal(!reveal());
      refetch();
    };
    insert(_el$9, () => reveal() ? "🙈 Hide sensitive" : "👁 Reveal sensitive");
    insert(_el$8, createComponent(A$1, {
      "class": "btn",
      get href() {
        return `/r/${params.id}/edit`;
      },
      children: "✎ Edit"
    }), _el$0);
    _el$0.$$click = del;
    insert(_el$, createComponent(Show, {
      get when() {
        return data();
      },
      get fallback() {
        return _tmpl$5$5();
      },
      get children() {
        return [createComponent(Show, {
          get when() {
            return fieldRows().length > 0;
          },
          get fallback() {
            return _tmpl$6$3();
          },
          get children() {
            var _el$1 = _tmpl$2$5();
            insert(_el$1, createComponent(For, {
              get each() {
                return fieldRows();
              },
              children: (row) => [(() => {
                var _el$15 = _tmpl$7$3();
                insert(_el$15, () => row.label);
                return _el$15;
              })(), (() => {
                var _el$16 = _tmpl$8$2();
                insert(_el$16, createComponent(Show, {
                  get when() {
                    return row.multiline;
                  },
                  get fallback() {
                    return (() => {
                      var _el$17 = _tmpl$9$1();
                      insert(_el$17, () => row.value);
                      return _el$17;
                    })();
                  },
                  get children() {
                    return createComponent(Markdown, {
                      get source() {
                        return row.value;
                      },
                      get titles() {
                        return titles() ?? {};
                      }
                    });
                  }
                }));
                createRenderEffect(() => className(_el$16, "detail-value" + (row.multiline ? " detail-multiline" : "") + (row.hidden ? " detail-masked" : "")));
                return _el$16;
              })()]
            }));
            return _el$1;
          }
        }), createComponent(Show, {
          get when() {
            return recordType() === "runbook";
          },
          get children() {
            return (() => {
              const steps = asSteps(data().steps);
              return createComponent(Show, {
                get when() {
                  return steps && steps.length > 0;
                },
                get children() {
                  var _el$18 = _tmpl$0$1(), _el$19 = _el$18.firstChild, _el$20 = _el$19.nextSibling;
                  insert(_el$20, createComponent(For, {
                    each: steps ?? [],
                    children: (s) => (() => {
                      var _el$21 = _tmpl$10$1(), _el$22 = _el$21.firstChild;
                      insert(_el$22, () => s.title);
                      insert(_el$21, createComponent(Markdown, {
                        get source() {
                          return s.body;
                        },
                        get titles() {
                          return titles() ?? {};
                        }
                      }), null);
                      insert(_el$21, createComponent(Show, {
                        get when() {
                          return s.status;
                        },
                        get children() {
                          var _el$23 = _tmpl$1$1();
                          insert(_el$23, () => s.status);
                          return _el$23;
                        }
                      }), null);
                      return _el$21;
                    })()
                  }));
                  return _el$18;
                }
              });
            })();
          }
        }), (() => {
          var _el$10 = _tmpl$3$5(), _el$11 = _el$10.firstChild, _el$12 = _el$11.nextSibling;
          insert(_el$12, () => JSON.stringify(data(), null, 2));
          return _el$10;
        })()];
      }
    }), null);
    return _el$;
  })();
}
delegateEvents(["click"]);
var _tmpl$$4 = /* @__PURE__ */ template(`<div class=page><header class=page-header><div><h1 class=page-title> </h1><p class=page-sub></p></div></header><form class=form><div class=form-grid></div><div class=form-actions><button type=submit class="btn btn-primary"></button><button type=button class="btn btn-ghost">Cancel`), _tmpl$2$4 = /* @__PURE__ */ template(`<textarea rows=4>`), _tmpl$3$4 = /* @__PURE__ */ template(`<div class=form-field><label>`), _tmpl$4$4 = /* @__PURE__ */ template(`<span class=form-hint>comma-separated, first is primary`), _tmpl$5$4 = /* @__PURE__ */ template(`<input>`);
const LIST_FIELDS = /* @__PURE__ */ new Set(["holders", "drivers", "users", "leaseholders", "occupants"]);
const PLACEHOLDERS = {
  login: {
    service: "Service name",
    username: "username or email",
    holders: "John Doe, Jane Doe",
    password: "Strong password",
    totp_secret: "Base32 secret",
    recovery_codes: "One code per line",
    url: "https://service.com/login",
    notes: "Recovery questions, etc."
  },
  document: {
    title: "Document title",
    document_type: "Lease, Contract, etc.",
    owner: "Whose document",
    number: "Document / ID number",
    issuer: "Issuing authority or agency",
    issued_on: "2020-06-15",
    expires_on: "2030-06-15",
    location: "Home safe, Bank box 47, etc.",
    notes: "Free-form notes"
  },
  identification: {
    holder: "Full legal name",
    id_type: "Driver's License, Passport, etc.",
    issuer: "Issuing state or agency",
    number: "ID number",
    country: "USA, Canada, etc.",
    class: "Class D, Real ID, etc.",
    issued_on: "2020-06-15",
    expires_on: "2030-06-15",
    notes: "Free-form notes"
  },
  insurance: {
    policy_type: "Auto, Renter's, Health, etc.",
    provider: "Carrier name",
    policy_number: "Policy number",
    group_number: "If applicable",
    member_id: "If applicable",
    holders: "John Doe, Jane Doe",
    beneficiary: "If applicable",
    insured_item: "2018 Honda Civic, 123 Main St, etc.",
    coverage: "$300,000 / $100,000",
    deductible: "$500",
    premium: "$120/mo",
    effective_on: "2025-01-01",
    renewal_on: "2026-01-01",
    agent: "Agent name & phone",
    claims_phone: "1-800-555-0100",
    notes: "Claims history, deductibles, etc."
  },
  health: {
    subject: "Whose record (self, child, etc.)",
    title: "Title",
    details: "JSON or free-form details"
  },
  bank_account: {
    bank: "Chase, Bank of America, etc.",
    account_type: "Checking, Savings, etc.",
    holders: "John Doe, Jane Doe",
    account_number: "1234567890",
    routing_number: "021000021",
    swift: "CHASUS33",
    branch: "Branch name or address",
    online_username: "Online banking username",
    online_url: "https://bank.com/login",
    notes: "Free-form notes"
  },
  credit_card: {
    issuer: "Chase, Capital One, etc.",
    network: "Visa, Mastercard, Amex, Discover",
    holders: "John Doe, Jane Doe",
    card_number: "4242 4242 4242 4242",
    expiration: "MM/YY",
    cvv: "123",
    pin: "0000",
    billing_address: "123 Main St, City, ST 12345",
    issuer_phone: "1-800-555-0100",
    issuer_url: "https://issuer.com",
    notes: "Free-form notes"
  },
  investment: {
    provider: "Fidelity, Vanguard, etc.",
    account_type: "Brokerage, 401k, IRA, etc.",
    holders: "John Doe, Jane Doe",
    notes: "Free-form notes"
  },
  income_source: {
    source: "Employer, client, etc.",
    income_type: "Salaried, Hourly, Contract, etc.",
    rate: "$20/hr, $87k/yr, etc.",
    schedule: "Biweekly, Monthly, etc.",
    per_payment: "$2,700",
    notes: "Free-form notes"
  },
  vehicle: {
    year: "YYYY",
    make_model: "Year Make Model",
    nickname: "Friendly name",
    drivers: "John Doe, Jane Doe",
    title_holder: "Bank name, if financed",
    vin: "17-character VIN",
    license_plate: "ABC-1234",
    notes: "Free-form notes"
  },
  residence: {
    address: "Street, City, ST ZIP",
    residence_type: "Rental, Owned, Family, etc.",
    landlord: "Landlord or property manager",
    leaseholders: "John Doe, Jane Doe",
    occupants: "John Doe, Jane Doe",
    rent: "$1,500/mo",
    deposit: "$1,500",
    notes: "Free-form notes"
  },
  phone: {
    device: "Device or line name",
    model: "iPhone 15 Pro, etc.",
    phone_number: "555-555-5555",
    carrier: "Verizon, AT&T, etc.",
    plan: "Plan name",
    users: "John Doe, Jane Doe",
    account_number: "Account number",
    pin: "Account PIN",
    notes: "Free-form notes"
  },
  address: {
    label: "Home, Work, Parents, etc.",
    street: "Street address",
    city: "City",
    region: "State or region",
    postal_code: "ZIP / postal code",
    country: "Country",
    notes: "Free-form notes"
  },
  contact: {
    name: "Full name",
    relationship: "Wife, Advisor, Friend, etc.",
    email: "name@example.com",
    phone: "555-555-5555",
    notes: "Free-form notes"
  },
  subscription: {
    service: "Service name",
    cost: "$9.99",
    cycle: "Monthly, Yearly, etc.",
    holders: "John Doe, Jane Doe",
    username: "Account username or email",
    notes: "Free-form notes"
  },
  infrastructure: {
    name: "Asset name",
    provider: "Provider name",
    asset_type: "VPS, DNS, Object storage, etc.",
    holders: "John Doe, Jane Doe",
    notes: "Free-form notes"
  },
  domain: {
    fqdn: "subdomain.example.com",
    points_to: "Where it resolves to",
    holders: "John Doe, Jane Doe",
    notes: "Free-form notes"
  },
  runbook: {
    title: "Runbook title",
    description: "What scenario triggers this runbook",
    steps: "Step title | Step body | status (one per line)",
    notes: "Free-form notes"
  },
  work_log: {
    date: "YYYY-MM-DD",
    project: "Project name",
    summary: "One-line summary",
    details: "Detailed notes",
    tags: "comma, separated, tags"
  },
  note: {
    title: "Note title",
    body: "Markdown supported",
    tags: "comma, separated, tags"
  }
};
function RecordForm() {
  const params = useParams();
  const nav = useNavigate();
  const isEdit = () => !!params.id;
  const [editType, setEditType] = createSignal(null);
  const recordType = () => {
    if (isEdit()) {
      const t = editType();
      if (t) return t;
    }
    return params.type ?? "note";
  };
  const schema = () => SCHEMAS[recordType()] ?? [];
  const [values, setValues] = createSignal({});
  const [busy, setBusy] = createSignal(false);
  onMount(async () => {
    if (!isEdit()) return;
    const rec = await api.showRecord(params.id, true);
    const t = rec.type;
    if (t) setEditType(t);
    const v2 = {};
    for (const f of schema()) {
      const val = rec[f.name];
      if (val == null) continue;
      if (f.name === "steps" && Array.isArray(val)) {
        v2[f.name] = val.map((s) => `${s.title} | ${s.body} | ${s.status ?? ""}`).join("\n");
      } else if (f.name === "details" && typeof val === "object") {
        v2[f.name] = JSON.stringify(val, null, 2);
      } else if (Array.isArray(val)) {
        v2[f.name] = val.join(", ");
      } else {
        v2[f.name] = String(val);
      }
    }
    setValues(v2);
  });
  function set(name, val) {
    setValues({
      ...values(),
      [name]: val
    });
  }
  async function save(e) {
    e.preventDefault();
    setBusy(true);
    try {
      const fields = {};
      const raw = values();
      for (const f of schema()) {
        const v2 = (raw[f.name] ?? "").trim();
        if (!v2 && !f.required) continue;
        if (f.kind === "number") {
          fields[f.name] = parseInt(v2, 10);
        } else if (f.name === "steps") {
          fields[f.name] = v2.split("\n").map((line) => {
            const [title, body, status2] = line.split("|").map((s) => s.trim());
            return {
              order: 0,
              title: title ?? "",
              body: body ?? "",
              status: status2 || null
            };
          });
        } else if (f.name === "tags") {
          fields[f.name] = v2.split(",").map((s) => s.trim()).filter(Boolean);
        } else if (LIST_FIELDS.has(f.name)) {
          fields[f.name] = v2.split(",").map((s) => s.trim()).filter(Boolean);
        } else if (f.name === "details") {
          try {
            fields[f.name] = JSON.parse(v2);
          } catch {
            fields[f.name] = v2;
          }
        } else {
          fields[f.name] = v2;
        }
      }
      if (isEdit()) {
        await api.updateRecord(params.id, fields);
        showToast("ok", "Record updated");
        nav(`/r/${params.id}`);
      } else {
        const id = await api.addRecord(recordType(), fields);
        showToast("ok", "Record created");
        nav(`/r/${id}`);
      }
    } catch (e2) {
      showToast("err", String(e2));
    } finally {
      setBusy(false);
    }
  }
  return (() => {
    var _el$ = _tmpl$$4(), _el$2 = _el$.firstChild, _el$3 = _el$2.firstChild, _el$4 = _el$3.firstChild, _el$5 = _el$4.firstChild, _el$6 = _el$4.nextSibling, _el$7 = _el$2.nextSibling, _el$8 = _el$7.firstChild, _el$9 = _el$8.nextSibling, _el$0 = _el$9.firstChild, _el$1 = _el$0.nextSibling;
    insert(_el$4, () => isEdit() ? "Edit" : "New", _el$5);
    insert(_el$4, () => RECORD_TYPES.find((t) => t.type === recordType())?.label ?? recordType(), null);
    insert(_el$6, () => isEdit() ? "Update this record's fields." : "Fill in the fields below.");
    _el$7.addEventListener("submit", save);
    insert(_el$8, createComponent(For, {
      get each() {
        return schema();
      },
      children: (f) => {
        const isList = LIST_FIELDS.has(f.name);
        const perType = PLACEHOLDERS[recordType()] ?? {};
        const placeholder = perType[f.name] ?? (f.kind === "password" ? "••••••" : "");
        return (() => {
          var _el$10 = _tmpl$3$4(), _el$11 = _el$10.firstChild;
          insert(_el$11, () => f.label, null);
          insert(_el$11, () => f.required ? " *" : "", null);
          insert(_el$11, isList && _tmpl$4$4(), null);
          insert(_el$10, createComponent(Show, {
            get when() {
              return f.multiline;
            },
            get fallback() {
              return (() => {
                var _el$14 = _tmpl$5$4();
                _el$14.$$input = (e) => set(f.name, e.currentTarget.value);
                setAttribute(_el$14, "placeholder", placeholder);
                createRenderEffect((_p$) => {
                  var _v$ = f.kind ?? "text", _v$2 = f.required;
                  _v$ !== _p$.e && setAttribute(_el$14, "type", _p$.e = _v$);
                  _v$2 !== _p$.t && (_el$14.required = _p$.t = _v$2);
                  return _p$;
                }, {
                  e: void 0,
                  t: void 0
                });
                createRenderEffect(() => _el$14.value = values()[f.name] ?? "");
                return _el$14;
              })();
            },
            get children() {
              var _el$12 = _tmpl$2$4();
              _el$12.$$input = (e) => set(f.name, e.currentTarget.value);
              setAttribute(_el$12, "placeholder", placeholder);
              createRenderEffect(() => _el$12.required = f.required);
              createRenderEffect(() => _el$12.value = values()[f.name] ?? "");
              return _el$12;
            }
          }), null);
          createRenderEffect((_$p) => style(_el$10, f.multiline ? "grid-column: 1 / -1" : "", _$p));
          return _el$10;
        })();
      }
    }));
    insert(_el$0, (() => {
      var _c$ = memo(() => !!busy());
      return () => _c$() ? "Saving…" : isEdit() ? "Save changes" : "Create record";
    })());
    _el$1.$$click = () => {
      if (isEdit()) {
        nav(`/r/${params.id}`);
      } else if (params.type) {
        nav(`/c/${params.type}`);
      } else {
        nav("/");
      }
    };
    createRenderEffect(() => _el$0.disabled = busy());
    return _el$;
  })();
}
delegateEvents(["click", "input"]);
var _tmpl$$3 = /* @__PURE__ */ template(`<div class="banner banner-warn">⚠️ Chain verification failed at entry <!>. The entries below are still displayed, but the chain is broken — usually because an older entry was written by an earlier version of Keepsake with a different hash function.`), _tmpl$2$3 = /* @__PURE__ */ template(`<p class=muted>Loading…`), _tmpl$3$3 = /* @__PURE__ */ template(`<div class=empty-state><div class=empty-state-emoji>⚠️</div><p class=empty-state-title>Failed to read audit log</p><p class=empty-state-sub>`), _tmpl$4$3 = /* @__PURE__ */ template(`<div class=empty-state><div class=empty-state-emoji>🛡</div><p class=empty-state-title>No audit entries yet</p><p class=empty-state-sub>Actions like unlocking the vault, adding records, and changing your password are recorded here.`), _tmpl$5$3 = /* @__PURE__ */ template(`<div class=table-wrap><table class=rows><thead><tr><th>seq</th><th>op</th><th>actor</th><th>target</th><th>details</th><th>ts</th></tr></thead><tbody>`), _tmpl$6$2 = /* @__PURE__ */ template(`<div class=page><header class=page-header><div><h1 class=page-title>🛡 Audit</h1><p class=page-sub>Append-only, hash-chained record of every change.</p></div><div class=page-actions><button class=btn>Refresh</button><button class=btn>Verify chain</button><button class="btn btn-danger">Reset chain`), _tmpl$7$2 = /* @__PURE__ */ template(`<div class=empty-state><div class=empty-state-emoji>🔒</div><p class=empty-state-title>Vault is locked</p><p class=empty-state-sub>Unlock the vault from the sidebar to view the audit log.`), _tmpl$8$1 = /* @__PURE__ */ template(`<tr class=audit-row><td></td><td><span class=op></span></td><td></td><td><code></code></td><td></td><td>`);
function AuditPage() {
  const [refreshTick, setRefreshTick] = createSignal(0);
  const [entries2] = createResource(() => refreshTick(), async () => {
    try {
      return await api.audit(false);
    } catch (e) {
      throw e;
    }
  });
  const [lastVerify, setLastVerify] = createSignal(null);
  async function doVerify() {
    try {
      const r = await api.audit(true);
      setLastVerify(r);
      if (r.ok) {
        showToast("ok", `Audit chain verified (${r.entries} entries)`);
      } else {
        showToast("err", `Chain verification failed at entry ${r.first_broken}`);
      }
    } catch (e) {
      showToast("err", String(e));
    }
  }
  function refresh() {
    setRefreshTick((n) => n + 1);
  }
  async function resetChain() {
    const ok = confirm("Reset audit chain?\n\nThis drops every entry before the first one whose hash doesn't match the current chain format, then re-chains the survivors starting from a new genesis. Destructive; cannot be undone.");
    if (!ok) return;
    try {
      const dropped = await api.rewriteAuditChain();
      showToast("ok", dropped === 0 ? "Audit chain was already valid" : `Dropped ${dropped} legacy entries and re-chained the rest`);
      setLastVerify(null);
      refresh();
    } catch (e) {
      showToast("err", String(e));
    }
  }
  const list2 = () => entries2() ?? [];
  return (() => {
    var _el$ = _tmpl$6$2(), _el$2 = _el$.firstChild, _el$3 = _el$2.firstChild, _el$4 = _el$3.nextSibling, _el$5 = _el$4.firstChild, _el$6 = _el$5.nextSibling, _el$7 = _el$6.nextSibling;
    _el$5.$$click = refresh;
    _el$6.$$click = doVerify;
    _el$7.$$click = resetChain;
    insert(_el$, createComponent(Show, {
      get when() {
        return state.status().unlocked;
      },
      get fallback() {
        return _tmpl$7$2();
      },
      get children() {
        return [createComponent(Show, {
          get when() {
            return memo(() => !!lastVerify())() && !lastVerify().ok;
          },
          get children() {
            var _el$8 = _tmpl$$3(), _el$9 = _el$8.firstChild, _el$1 = _el$9.nextSibling;
            _el$1.nextSibling;
            insert(_el$8, () => lastVerify().first_broken, _el$1);
            return _el$8;
          }
        }), createComponent(Switch, {
          get children() {
            return [createComponent(Match, {
              get when() {
                return entries2.loading;
              },
              get children() {
                return _tmpl$2$3();
              }
            }), createComponent(Match, {
              get when() {
                return entries2.error;
              },
              get children() {
                var _el$11 = _tmpl$3$3(), _el$12 = _el$11.firstChild, _el$13 = _el$12.nextSibling, _el$14 = _el$13.nextSibling;
                insert(_el$14, () => String(entries2.error));
                return _el$11;
              }
            }), createComponent(Match, {
              get when() {
                return list2().length === 0;
              },
              get children() {
                return _tmpl$4$3();
              }
            }), createComponent(Match, {
              get when() {
                return list2().length > 0;
              },
              get children() {
                var _el$16 = _tmpl$5$3(), _el$17 = _el$16.firstChild, _el$18 = _el$17.firstChild, _el$19 = _el$18.nextSibling;
                insert(_el$19, createComponent(For, {
                  get each() {
                    return list2();
                  },
                  children: (e) => (() => {
                    var _el$21 = _tmpl$8$1(), _el$22 = _el$21.firstChild, _el$23 = _el$22.nextSibling, _el$24 = _el$23.firstChild, _el$25 = _el$23.nextSibling, _el$26 = _el$25.nextSibling, _el$27 = _el$26.firstChild, _el$28 = _el$26.nextSibling, _el$29 = _el$28.nextSibling;
                    insert(_el$22, () => e.seq);
                    insert(_el$24, () => e.op);
                    insert(_el$25, () => e.actor);
                    insert(_el$27, () => e.target_id ?? "");
                    insert(_el$28, () => e.details ?? "");
                    insert(_el$29, () => new Date(e.ts).toLocaleString());
                    return _el$21;
                  })()
                }));
                return _el$16;
              }
            })];
          }
        })];
      }
    }), null);
    return _el$;
  })();
}
delegateEvents(["click"]);
var _tmpl$$2 = /* @__PURE__ */ template(`<div class=page><header class=page-header><div><h1 class=page-title>Settings</h1><p class=page-sub>Vault, security, and backup.`), _tmpl$2$2 = /* @__PURE__ */ template(`<span class="badge accent">`), _tmpl$3$2 = /* @__PURE__ */ template(`<div class=settings-section><h2>Vault</h2><dl class=dl><dt>Path</dt><dd><code></code></dd><dt>Current user</dt><dd>`), _tmpl$4$2 = /* @__PURE__ */ template(`<div class=form-actions style=border-top:none;padding-top:0><button class="btn btn-primary">⤓ Export vault</button><button class=btn>⤒ Import bundle`), _tmpl$5$2 = /* @__PURE__ */ template(`<div class=form-field><label>bundle (copy this and save somewhere safe)</label><textarea rows=6 readonly>`), _tmpl$6$1 = /* @__PURE__ */ template(`<form class=backup-form><div class=form-field><label>export passphrase</label><input type=password placeholder="A long, written-down string"required></div><div class=form-field><label>confirm</label><input type=password required></div><div class=form-actions><button type=submit class="btn btn-primary"></button><button type=button class="btn btn-ghost">Cancel`), _tmpl$7$1 = /* @__PURE__ */ template(`<form class=backup-form><div class=form-field><label>bundle passphrase</label><input type=password required></div><div class=form-field><label>bundle bytes (JSON array)</label><textarea rows=6 placeholder="Paste the [12, 34, 56, ...] array from your export"required></textarea></div><div class=form-actions><button type=submit class="btn btn-primary"></button><button type=button class="btn btn-ghost">Cancel`), _tmpl$8 = /* @__PURE__ */ template(`<div class=settings-section id=export><h2>Backup</h2><p class=muted-small style="margin:0 0 0.75rem 0">Export the entire vault to an encrypted <code>.ksk</code> bundle. The bundle is sealed under a passphrase you choose — it can be different from your daily password. Save the bundle to offline media for recovery.`), _tmpl$9 = /* @__PURE__ */ template(`<button class=btn>+ Add user`), _tmpl$0 = /* @__PURE__ */ template(`<form class=add-user-form><div class=form-field><label>username</label><input placeholder="e.g. dahlia"required></div><div class=form-field><label>password</label><input type=password placeholder="strong password"required></div><div class=form-actions><button type=submit class="btn btn-primary"></button><button type=button class="btn btn-ghost">Cancel`), _tmpl$1 = /* @__PURE__ */ template(`<ul class=user-list>`), _tmpl$10 = /* @__PURE__ */ template(`<div class=settings-section><div class=settings-section-header><h2>Users on this device`), _tmpl$11 = /* @__PURE__ */ template(`<button class="btn btn-ghost">Cancel`), _tmpl$12 = /* @__PURE__ */ template(`<p class=muted>No users on this device.`), _tmpl$13 = /* @__PURE__ */ template(`<span class=muted-small style=margin-left:0.5rem>current`), _tmpl$14 = /* @__PURE__ */ template(`<button class="btn btn-ghost"title="Remove from this device">Remove`), _tmpl$15 = /* @__PURE__ */ template(`<li class=user-row><span class=user-name>`), _tmpl$16 = /* @__PURE__ */ template(`<span class=badge>`), _tmpl$17 = /* @__PURE__ */ template(`<div class=settings-section><h2>Change password</h2><p class=muted-small style="margin:0 0 1rem 0">Re-seals the vault key under a new password. The vault key itself stays the same.</p><form class=change-pw-form><div class=form-field><label>new password</label><input type=password required></div><div class=form-field><label>confirm</label><input type=password required></div><div class=form-actions><button type=submit class="btn btn-primary">`), _tmpl$18 = /* @__PURE__ */ template(`<div class=settings-section><h2>Security</h2><p style=margin:0>All data is end-to-end encrypted. The master password is the only thing protecting the vault; if you lose it, recovery requires a<code> .ksk</code> export.</p><p class=muted-small style="margin:0.5rem 0 0 0">See <code>docs/threat-model.md</code> in the source tree for the full threat model.`);
function Settings() {
  onMount(async () => {
    await refreshUsers();
  });
  return (() => {
    var _el$ = _tmpl$$2();
    _el$.firstChild;
    insert(_el$, createComponent(VaultInfo, {}), null);
    insert(_el$, createComponent(Backup, {}), null);
    insert(_el$, createComponent(Users, {}), null);
    insert(_el$, createComponent(ChangePassword, {}), null);
    insert(_el$, createComponent(Security, {}), null);
    return _el$;
  })();
}
function VaultInfo() {
  return (() => {
    var _el$3 = _tmpl$3$2(), _el$4 = _el$3.firstChild, _el$5 = _el$4.nextSibling, _el$6 = _el$5.firstChild, _el$7 = _el$6.nextSibling, _el$8 = _el$7.firstChild, _el$9 = _el$7.nextSibling, _el$0 = _el$9.nextSibling;
    insert(_el$8, () => state.vaultPath() || "—");
    insert(_el$0, createComponent(Show, {
      get when() {
        return state.status().username;
      },
      fallback: "—",
      get children() {
        var _el$1 = _tmpl$2$2();
        insert(_el$1, () => state.status().username);
        return _el$1;
      }
    }));
    return _el$3;
  })();
}
function Backup() {
  const [mode, setMode] = createSignal("idle");
  const [pass, setPass] = createSignal("");
  const [confirm2, setConfirm] = createSignal("");
  const [bytes, setBytes] = createSignal("");
  const [busy, setBusy] = createSignal(false);
  async function doExport(e) {
    e.preventDefault();
    if (pass() !== confirm2()) {
      showToast("err", "Passphrases do not match");
      return;
    }
    setBusy(true);
    try {
      const data = await api.exportBundle(pass());
      setBytes(JSON.stringify(data));
      showToast("ok", `Bundle generated (${data.length} bytes). Copy and save to a safe location.`);
    } catch (err) {
      showToast("err", String(err));
    } finally {
      setBusy(false);
    }
  }
  async function doImport(e) {
    e.preventDefault();
    if (!bytes().trim()) {
      showToast("err", "Paste bundle bytes first");
      return;
    }
    let arr;
    try {
      arr = JSON.parse(bytes().trim());
      if (!Array.isArray(arr)) throw new Error("not an array");
    } catch (err) {
      showToast("err", `Invalid bundle: ${err}`);
      return;
    }
    setBusy(true);
    try {
      await api.importBundle(arr, pass());
      showToast("ok", "Bundle imported");
      setMode("idle");
    } catch (err) {
      showToast("err", String(err));
    } finally {
      setBusy(false);
    }
  }
  return (() => {
    var _el$10 = _tmpl$8(), _el$11 = _el$10.firstChild;
    _el$11.nextSibling;
    insert(_el$10, createComponent(Show, {
      get when() {
        return mode() === "idle";
      },
      get children() {
        var _el$13 = _tmpl$4$2(), _el$14 = _el$13.firstChild, _el$15 = _el$14.nextSibling;
        _el$14.$$click = () => setMode("export");
        _el$15.$$click = () => setMode("import");
        return _el$13;
      }
    }), null);
    insert(_el$10, createComponent(Show, {
      get when() {
        return mode() === "export";
      },
      get children() {
        var _el$16 = _tmpl$6$1(), _el$17 = _el$16.firstChild, _el$18 = _el$17.firstChild, _el$19 = _el$18.nextSibling, _el$20 = _el$17.nextSibling, _el$21 = _el$20.firstChild, _el$22 = _el$21.nextSibling, _el$26 = _el$20.nextSibling, _el$27 = _el$26.firstChild, _el$28 = _el$27.nextSibling;
        _el$16.addEventListener("submit", doExport);
        _el$19.$$input = (e) => setPass(e.currentTarget.value);
        _el$22.$$input = (e) => setConfirm(e.currentTarget.value);
        insert(_el$16, createComponent(Show, {
          get when() {
            return bytes();
          },
          get children() {
            var _el$23 = _tmpl$5$2(), _el$24 = _el$23.firstChild, _el$25 = _el$24.nextSibling;
            _el$25.$$click = (e) => e.currentTarget.select();
            createRenderEffect(() => _el$25.value = bytes());
            return _el$23;
          }
        }), _el$26);
        insert(_el$27, () => busy() ? "Generating…" : "Generate bundle");
        _el$28.$$click = () => {
          setMode("idle");
          setPass("");
          setConfirm("");
          setBytes("");
        };
        createRenderEffect(() => _el$27.disabled = busy());
        createRenderEffect(() => _el$19.value = pass());
        createRenderEffect(() => _el$22.value = confirm2());
        return _el$16;
      }
    }), null);
    insert(_el$10, createComponent(Show, {
      get when() {
        return mode() === "import";
      },
      get children() {
        var _el$29 = _tmpl$7$1(), _el$30 = _el$29.firstChild, _el$31 = _el$30.firstChild, _el$32 = _el$31.nextSibling, _el$33 = _el$30.nextSibling, _el$34 = _el$33.firstChild, _el$35 = _el$34.nextSibling, _el$36 = _el$33.nextSibling, _el$37 = _el$36.firstChild, _el$38 = _el$37.nextSibling;
        _el$29.addEventListener("submit", doImport);
        _el$32.$$input = (e) => setPass(e.currentTarget.value);
        _el$35.$$input = (e) => setBytes(e.currentTarget.value);
        insert(_el$37, () => busy() ? "Importing…" : "Import bundle");
        _el$38.$$click = () => {
          setMode("idle");
          setPass("");
          setBytes("");
        };
        createRenderEffect(() => _el$37.disabled = busy());
        createRenderEffect(() => _el$32.value = pass());
        createRenderEffect(() => _el$35.value = bytes());
        return _el$29;
      }
    }), null);
    return _el$10;
  })();
}
function Users() {
  const [showForm, setShowForm] = createSignal(false);
  const [newName, setNewName] = createSignal("");
  const [newPw, setNewPw] = createSignal("");
  const [busy, setBusy] = createSignal(false);
  async function addUser(e) {
    e.preventDefault();
    setBusy(true);
    try {
      await api.addUser(newName(), newPw());
      setNewName("");
      setNewPw("");
      setShowForm(false);
      await refreshUsers();
      showToast("ok", `Added user ${newName()}`);
    } catch (err) {
      showToast("err", String(err));
    } finally {
      setBusy(false);
    }
  }
  async function removeUser(username) {
    if (!confirm(`Remove user "${username}" from this device? They can be re-added later with their password.`)) {
      return;
    }
    try {
      await api.removeUser(username);
      await refreshUsers();
      showToast("ok", `Removed user ${username}`);
    } catch (err) {
      showToast("err", String(err));
    }
  }
  return (() => {
    var _el$39 = _tmpl$10(), _el$40 = _el$39.firstChild;
    _el$40.firstChild;
    insert(_el$40, createComponent(Show, {
      get when() {
        return !showForm();
      },
      get fallback() {
        return (() => {
          var _el$54 = _tmpl$11();
          _el$54.$$click = () => setShowForm(false);
          return _el$54;
        })();
      },
      get children() {
        var _el$42 = _tmpl$9();
        _el$42.$$click = () => setShowForm(true);
        return _el$42;
      }
    }), null);
    insert(_el$39, createComponent(Show, {
      get when() {
        return showForm();
      },
      get children() {
        var _el$43 = _tmpl$0(), _el$44 = _el$43.firstChild, _el$45 = _el$44.firstChild, _el$46 = _el$45.nextSibling, _el$47 = _el$44.nextSibling, _el$48 = _el$47.firstChild, _el$49 = _el$48.nextSibling, _el$50 = _el$47.nextSibling, _el$51 = _el$50.firstChild, _el$52 = _el$51.nextSibling;
        _el$43.addEventListener("submit", addUser);
        _el$46.$$input = (e) => setNewName(e.currentTarget.value);
        _el$49.$$input = (e) => setNewPw(e.currentTarget.value);
        insert(_el$51, () => busy() ? "Adding…" : "Add user");
        _el$52.$$click = () => setShowForm(false);
        createRenderEffect(() => _el$51.disabled = busy());
        createRenderEffect(() => _el$46.value = newName());
        createRenderEffect(() => _el$49.value = newPw());
        return _el$43;
      }
    }), null);
    insert(_el$39, createComponent(Show, {
      get when() {
        return state.users().length > 0;
      },
      get fallback() {
        return createComponent(Show, {
          get when() {
            return !showForm();
          },
          get children() {
            return _tmpl$12();
          }
        });
      },
      get children() {
        var _el$53 = _tmpl$1();
        insert(_el$53, createComponent(For, {
          get each() {
            return state.users();
          },
          children: (u) => (() => {
            var _el$56 = _tmpl$15(), _el$57 = _el$56.firstChild;
            insert(_el$57, createComponent(Show, {
              get when() {
                return u === state.status().username;
              },
              get fallback() {
                return (() => {
                  var _el$61 = _tmpl$16();
                  insert(_el$61, u);
                  return _el$61;
                })();
              },
              get children() {
                return [(() => {
                  var _el$58 = _tmpl$2$2();
                  insert(_el$58, u);
                  return _el$58;
                })(), _tmpl$13()];
              }
            }));
            insert(_el$56, createComponent(Show, {
              get when() {
                return u !== state.status().username;
              },
              get children() {
                var _el$60 = _tmpl$14();
                _el$60.$$click = () => removeUser(u);
                return _el$60;
              }
            }), null);
            return _el$56;
          })()
        }));
        return _el$53;
      }
    }), null);
    return _el$39;
  })();
}
function ChangePassword() {
  const [next, setNext] = createSignal("");
  const [confirm2, setConfirm] = createSignal("");
  const [busy, setBusy] = createSignal(false);
  async function submit(e) {
    e.preventDefault();
    if (next() !== confirm2()) {
      showToast("err", "New passwords do not match");
      return;
    }
    setBusy(true);
    try {
      await api.changePassword(next());
      setNext("");
      setConfirm("");
      showToast("ok", "Password changed");
    } catch (err) {
      showToast("err", String(err));
    } finally {
      setBusy(false);
    }
  }
  return (() => {
    var _el$62 = _tmpl$17(), _el$63 = _el$62.firstChild, _el$64 = _el$63.nextSibling, _el$65 = _el$64.nextSibling, _el$66 = _el$65.firstChild, _el$67 = _el$66.firstChild, _el$68 = _el$67.nextSibling, _el$69 = _el$66.nextSibling, _el$70 = _el$69.firstChild, _el$71 = _el$70.nextSibling, _el$72 = _el$69.nextSibling, _el$73 = _el$72.firstChild;
    _el$65.addEventListener("submit", submit);
    _el$68.$$input = (e) => setNext(e.currentTarget.value);
    _el$71.$$input = (e) => setConfirm(e.currentTarget.value);
    insert(_el$73, () => busy() ? "Changing…" : "Change password");
    createRenderEffect(() => _el$73.disabled = busy());
    createRenderEffect(() => _el$68.value = next());
    createRenderEffect(() => _el$71.value = confirm2());
    return _el$62;
  })();
}
function Security() {
  return _tmpl$18();
}
delegateEvents(["click", "input"]);
var _tmpl$$1 = /* @__PURE__ */ template(`<p class=muted>Unlock the vault to use sync.`), _tmpl$2$1 = /* @__PURE__ */ template(`<div class=form-actions style=margin-top:0.5rem><button type=button class=btn title="Push every local record to the server"></button><button type=button class=btn title="Pull remote changes and merge them locally">`), _tmpl$3$1 = /* @__PURE__ */ template(`<div class=form-field><label>server URL</label><div style=display:flex;gap:0.5rem><input type=text readonly><button type=button class=btn>Copy`), _tmpl$4$1 = /* @__PURE__ */ template(`<div class=settings-row style=margin-top:1rem><p class=muted-small style="margin:0 0 0.5rem 0">Share these out-of-band to set up another device:</p><div class=form-field><label>vault id</label><div style=display:flex;gap:0.5rem><input type=text readonly><button type=button class=btn>Copy</button></div></div><div class=form-field><label>passphrase</label><div style=display:flex;gap:0.5rem><input type=text readonly><button type=button class=btn>Copy</button></div></div><p class=muted-small style="margin:0.5rem 0 0 0">Anyone with the vault id and passphrase can read and write every record in the vault. Treat them like a shared key.`), _tmpl$5$1 = /* @__PURE__ */ template(`<div class=settings-section><form class=form style=background:transparent;border:none;padding:0><div class=form-field><label>server base URL</label><input type=url placeholder=https://sync.example.com></div><div class=form-field><label>vault id</label><input type=text placeholder=family list=known-vault-ids><datalist id=known-vault-ids></datalist></div><div class=form-field><label>shared passphrase (only needed to set up or rotate)</label><input type=password placeholder="used to derive the shared sync key"></div><div class=form-actions><button type=submit class=btn>Save URL</button><button type=button class="btn btn-primary"></button><button type=button class=btn></button><button type=button class=btn>`), _tmpl$6 = /* @__PURE__ */ template(`<div class=page><header class=page-header><div><h1 class=page-title>⇄ Sync</h1><p class=page-sub>End-to-end-encrypted. The server stores doubly-sealed ciphertext (vault key inner, shared sync key outer). Share the vault id and passphrase out-of-band to add another device.`), _tmpl$7 = /* @__PURE__ */ template(`<option>`);
function SyncPage() {
  const [url, setUrl] = createSignal(state.syncUrl());
  const [vaultId, setVaultIdRaw] = createSignal(state.syncVaultId());
  const [passphrase, setPassphrase] = createSignal("");
  const [busy, setBusy] = createSignal("");
  const [revealed, setRevealed] = createSignal(null);
  const setVaultId = (v2) => {
    setVaultIdRaw(v2);
    state.setSyncVaultId(v2);
  };
  const [syncIds, {
    refetch: refetchSyncIds
  }] = createResource(() => state.status().unlocked && state.status().username, async (unlocked) => {
    if (!unlocked) return [];
    return api.listSharedSyncs();
  });
  createEffect(() => {
    const ids = syncIds();
    if (!ids) return;
    const current = vaultId();
    if (current !== "" && ids.includes(current)) return;
    if (ids.length > 0) {
      setVaultId(ids[0]);
    }
  });
  const isConfigured = () => {
    const ids = syncIds() ?? [];
    const v2 = vaultId();
    return v2 !== "" && ids.includes(v2);
  };
  async function save(e) {
    e.preventDefault();
    state.setSyncUrl(url());
    showToast("ok", "Sync URL saved");
  }
  async function push() {
    if (!isConfigured()) {
      showToast("err", "Set up shared sync for this vault id first");
      return;
    }
    setBusy("push");
    try {
      const n = await api.syncPush(url(), vaultId());
      showToast("ok", `Pushed ${n} record(s)`);
    } catch (e) {
      showToast("err", String(e));
    } finally {
      setBusy("");
    }
  }
  async function pull() {
    if (!isConfigured()) {
      showToast("err", "Set up shared sync for this vault id first");
      return;
    }
    setBusy("pull");
    try {
      const n = await api.syncPull(url(), vaultId());
      showToast("ok", `Pulled ${n} record(s)`);
    } catch (e) {
      showToast("err", String(e));
    } finally {
      setBusy("");
    }
  }
  async function setupShared() {
    if (!vaultId() || !passphrase()) {
      showToast("err", "Both vault id and passphrase are required");
      return;
    }
    setBusy("setup");
    try {
      state.setSyncUrl(url());
      await api.setupSharedSync(vaultId(), passphrase(), url() || null);
      setPassphrase("");
      setRevealed(null);
      await refetchSyncIds();
      showToast("ok", `Shared sync set up for '${vaultId()}'`);
    } catch (e) {
      showToast("err", String(e));
    } finally {
      setBusy("");
    }
  }
  async function revealShared() {
    if (!vaultId()) {
      showToast("err", "Enter a vault id first");
      return;
    }
    setBusy("reveal");
    try {
      const r = await api.revealSharedSync(vaultId());
      setRevealed(r);
      showToast("ok", `Revealed setup for '${r[0]}'`);
    } catch (e) {
      showToast("err", String(e));
    } finally {
      setBusy("");
    }
  }
  async function deleteShared() {
    if (!vaultId()) {
      showToast("err", "Enter a vault id first");
      return;
    }
    setBusy("delete");
    try {
      await api.deleteSharedSync(vaultId());
      setRevealed(null);
      await refetchSyncIds();
      showToast("ok", `Deleted shared sync '${vaultId()}'`);
    } catch (e) {
      showToast("err", String(e));
    } finally {
      setBusy("");
    }
  }
  function copyToClipboard(s) {
    if (typeof navigator !== "undefined" && navigator.clipboard) {
      navigator.clipboard.writeText(s).then(() => showToast("ok", "Copied"), () => showToast("err", "Clipboard copy failed"));
    } else {
      showToast("err", "Clipboard not available");
    }
  }
  return (() => {
    var _el$ = _tmpl$6();
    _el$.firstChild;
    insert(_el$, createComponent(Show, {
      get when() {
        return !state.status().unlocked;
      },
      get children() {
        return _tmpl$$1();
      }
    }), null);
    insert(_el$, createComponent(Show, {
      get when() {
        return state.status().unlocked;
      },
      get children() {
        var _el$4 = _tmpl$5$1(), _el$5 = _el$4.firstChild, _el$6 = _el$5.firstChild, _el$7 = _el$6.firstChild, _el$8 = _el$7.nextSibling, _el$9 = _el$6.nextSibling, _el$0 = _el$9.firstChild, _el$1 = _el$0.nextSibling, _el$10 = _el$1.nextSibling, _el$11 = _el$9.nextSibling, _el$12 = _el$11.firstChild, _el$13 = _el$12.nextSibling, _el$14 = _el$11.nextSibling, _el$15 = _el$14.firstChild, _el$16 = _el$15.nextSibling, _el$17 = _el$16.nextSibling, _el$18 = _el$17.nextSibling;
        _el$5.addEventListener("submit", save);
        _el$8.$$input = (e) => setUrl(e.currentTarget.value);
        _el$1.$$input = (e) => setVaultId(e.currentTarget.value);
        insert(_el$10, createComponent(For, {
          get each() {
            return syncIds() ?? [];
          },
          children: (id) => (() => {
            var _el$40 = _tmpl$7();
            _el$40.value = id;
            return _el$40;
          })()
        }));
        _el$13.$$input = (e) => setPassphrase(e.currentTarget.value);
        _el$16.$$click = setupShared;
        insert(_el$16, () => busy() === "setup" ? "Saving…" : "Set up / rotate");
        _el$17.$$click = revealShared;
        insert(_el$17, () => busy() === "reveal" ? "…" : "Show setup");
        _el$18.$$click = deleteShared;
        insert(_el$18, () => busy() === "delete" ? "…" : "Delete");
        insert(_el$5, createComponent(Show, {
          get when() {
            return isConfigured();
          },
          get children() {
            var _el$19 = _tmpl$2$1(), _el$20 = _el$19.firstChild, _el$21 = _el$20.nextSibling;
            _el$20.$$click = push;
            insert(_el$20, () => busy() === "push" ? "Pushing…" : "Push");
            _el$21.$$click = pull;
            insert(_el$21, () => busy() === "pull" ? "Pulling…" : "Pull");
            createRenderEffect((_p$) => {
              var _v$ = busy() !== "", _v$2 = busy() !== "";
              _v$ !== _p$.e && (_el$20.disabled = _p$.e = _v$);
              _v$2 !== _p$.t && (_el$21.disabled = _p$.t = _v$2);
              return _p$;
            }, {
              e: void 0,
              t: void 0
            });
            return _el$19;
          }
        }), null);
        insert(_el$4, createComponent(Show, {
          get when() {
            return revealed();
          },
          get children() {
            var _el$22 = _tmpl$4$1(), _el$23 = _el$22.firstChild, _el$24 = _el$23.nextSibling, _el$25 = _el$24.firstChild, _el$26 = _el$25.nextSibling, _el$27 = _el$26.firstChild, _el$28 = _el$27.nextSibling, _el$29 = _el$24.nextSibling, _el$30 = _el$29.firstChild, _el$31 = _el$30.nextSibling, _el$32 = _el$31.firstChild, _el$33 = _el$32.nextSibling, _el$39 = _el$29.nextSibling;
            _el$28.$$click = () => copyToClipboard(revealed()[0]);
            _el$33.$$click = () => copyToClipboard(revealed()[1]);
            insert(_el$22, createComponent(Show, {
              get when() {
                return revealed()[2];
              },
              get children() {
                var _el$34 = _tmpl$3$1(), _el$35 = _el$34.firstChild, _el$36 = _el$35.nextSibling, _el$37 = _el$36.firstChild, _el$38 = _el$37.nextSibling;
                _el$38.$$click = () => copyToClipboard(revealed()[2]);
                createRenderEffect(() => _el$37.value = revealed()[2]);
                return _el$34;
              }
            }), _el$39);
            createRenderEffect(() => _el$27.value = revealed()[0]);
            createRenderEffect(() => _el$32.value = revealed()[1]);
            return _el$22;
          }
        }), null);
        createRenderEffect((_p$) => {
          var _v$3 = busy() !== "", _v$4 = busy() !== "" || !vaultId() || !passphrase(), _v$5 = busy() !== "" || !vaultId(), _v$6 = busy() !== "" || !vaultId();
          _v$3 !== _p$.e && (_el$15.disabled = _p$.e = _v$3);
          _v$4 !== _p$.t && (_el$16.disabled = _p$.t = _v$4);
          _v$5 !== _p$.a && (_el$17.disabled = _p$.a = _v$5);
          _v$6 !== _p$.o && (_el$18.disabled = _p$.o = _v$6);
          return _p$;
        }, {
          e: void 0,
          t: void 0,
          a: void 0,
          o: void 0
        });
        createRenderEffect(() => _el$8.value = url());
        createRenderEffect(() => _el$1.value = vaultId());
        createRenderEffect(() => _el$13.value = passphrase());
        return _el$4;
      }
    }), null);
    return _el$;
  })();
}
delegateEvents(["input", "click"]);
var _tmpl$ = /* @__PURE__ */ template(`<div class=insights-list>`), _tmpl$2 = /* @__PURE__ */ template(`<div class=page><header class=page-header><div><h1 class=page-title>📈 Insights</h1><p class=page-sub>Things worth looking at — expiring items, missing fields, gaps in your records.</p></div><div class=page-actions><button class=btn>↻ Refresh`), _tmpl$3 = /* @__PURE__ */ template(`<p class=muted>Scanning vault…`), _tmpl$4 = /* @__PURE__ */ template(`<div class=table-wrap><div class=empty-state><div class=empty-state-emoji>✨</div><p class=empty-state-title>Nothing to flag right now</p><p class=empty-state-sub>Your vault is in good shape.`), _tmpl$5 = /* @__PURE__ */ template(`<div><div class=insight-icon></div><div class=insight-body><div class=insight-title></div><div class=insight-detail>`);
function Insights() {
  const [insights, {
    refetch
  }] = createResource(async () => {
    return await generateInsights();
  });
  onMount(() => {
    refetch();
  });
  function refetchOnFocus() {
    refetch();
  }
  if (typeof window !== "undefined") {
    window.addEventListener("focus", refetchOnFocus);
  }
  return (() => {
    var _el$ = _tmpl$2(), _el$2 = _el$.firstChild, _el$3 = _el$2.firstChild, _el$4 = _el$3.nextSibling, _el$5 = _el$4.firstChild;
    _el$5.$$click = () => refetch();
    insert(_el$, createComponent(Show, {
      get when() {
        return insights();
      },
      get fallback() {
        return _tmpl$3();
      },
      get children() {
        return createComponent(Show, {
          get when() {
            return insights().length > 0;
          },
          get fallback() {
            return _tmpl$4();
          },
          get children() {
            var _el$6 = _tmpl$();
            insert(_el$6, createComponent(For, {
              get each() {
                return insights();
              },
              children: (i) => createComponent(InsightCard, {
                insight: i
              })
            }));
            return _el$6;
          }
        });
      }
    }), null);
    return _el$;
  })();
}
function InsightCard(props) {
  const i = () => props.insight;
  return (() => {
    var _el$9 = _tmpl$5(), _el$0 = _el$9.firstChild, _el$1 = _el$0.nextSibling, _el$10 = _el$1.firstChild, _el$11 = _el$10.nextSibling;
    insert(_el$0, () => iconFor(i().severity));
    insert(_el$10, () => i().title);
    insert(_el$11, () => i().detail);
    insert(_el$9, createComponent(Show, {
      get when() {
        return i().to;
      },
      get children() {
        return createComponent(A$1, {
          "class": "insight-action",
          get href() {
            return i().to;
          },
          children: "Open →"
        });
      }
    }), null);
    createRenderEffect(() => className(_el$9, `insight insight-${i().severity}`));
    return _el$9;
  })();
}
function iconFor(s) {
  if (s === "warn") return "⚠";
  if (s === "ok") return "✓";
  return "ⓘ";
}
delegateEvents(["click"]);
const root = document.getElementById("root");
if (root) {
  render(() => createComponent(Router, {
    root: App,
    get children() {
      return [createComponent(Route, {
        path: "/",
        component: Dashboard
      }), createComponent(Route, {
        path: "/c/:type",
        component: Category
      }), createComponent(Route, {
        path: "/r/:id",
        component: RecordDetail
      }), createComponent(Route, {
        path: "/c/:type/new",
        component: RecordForm
      }), createComponent(Route, {
        path: "/r/:id/edit",
        component: RecordForm
      }), createComponent(Route, {
        path: "/audit",
        component: AuditPage
      }), createComponent(Route, {
        path: "/sync",
        component: SyncPage
      }), createComponent(Route, {
        path: "/insights",
        component: Insights
      }), createComponent(Route, {
        path: "/settings",
        component: Settings
      })];
    }
  }), root);
}
export {
  Channel as C,
  Resource as R,
  invoke as i
};
//# sourceMappingURL=index-DES0A9eA.js.map
