(ns quayside.units.clock)

#?(:clj (defn now [] (System/currentTimeMillis))
   :cljs (defn now [] (.now js/Date)))

#?@(:clj [(def zone "UTC") (defn- offset [] 0)])

(defn elapsed [start]
  (- (now) start))
