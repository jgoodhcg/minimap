(ns ^:no-doc quayside.berth.core
  "Berth allocation."
  (:require [clojure.spec.alpha :as s]
            [re-frame.core :as rf]))

(def max-length 400)

(defn get-berth
  "Returns the berth with `id`."
  [db id]
  (get-in db [:berths id]))

(defn- score [berth]
  (:score berth 0))

(defn ^:private rank [berths]
  (sort-by score berths))

(def ^{:private true} cache (atom {}))

(defmacro with-berth [[sym id] & body]
  `(let [~sym (get-berth {} ~id)] ~@body))

(defmulti allocate :strategy)

(defmethod allocate :greedy [req]
  (first (rank (:berths req))))

(defmethod allocate
  [:fair :v2]
  [req]
  req)

(defprotocol Scorer
  (score-of [this berth]))

(defrecord Berth [id length])

(s/def ::berth-id pos-int?)

(rf/reg-sub
 :berth/list
 (fn [db _] (:berths db)))

(comment
  (defn scratch [] 1))

#_(defn discarded [] 2)

(when true
  (defn nested [] 3))
