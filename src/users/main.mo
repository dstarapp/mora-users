import Types "./types";
import Helper "./helper";
import Planet "./planet";
import Bool "mo:base/Bool";
import Principal "mo:base/Principal";
import Text "mo:base/Text";
import Option "mo:base/Option";
import TrieMap "mo:base/TrieMap";
import Queue "mo:mutable-queue/Queue";
import Iter "mo:base/Iter";
import Time "mo:base/Time";
import Buffer "mo:base/Buffer";
import Cycles "mo:base/ExperimentalCycles";
import Prim "mo:prim";
import Nat "mo:base/Nat";
import Nat64 "mo:base/Nat64";

shared ({ caller = owner_ }) actor class UserActor(helperid : Principal) = this {
  type UserV3 = Types.UserV3;
  type NFT = Types.NFT;
  type Attribute = Types.Attribute;
  type UserInfo = Types.UserInfo;
  type PlanetArgs = Types.PlanetArgs;
  type PlanetMsg = Types.PlanetMsg;
  type Collection = Types.Collection;
  type QueryCommonReq = Types.QueryCommonReq;
  type QueryCollectionResp = Types.QueryCollectionResp;

  private stable var owner : Principal = owner_;
  private stable var launchHelperID : Principal = helperid;

  private var users_v3 = TrieMap.TrieMap<Principal, UserV3>(Principal.equal, Principal.hash);
  private stable var stable_users_v3 : [(Principal, UserV3)] = [];

  public query func wallet_balance() : async Nat {
    return Cycles.balance();
  };

  //cycles deposit
  public func wallet_receive() : async { accepted : Nat64 } {
    let available = Cycles.available();
    let accepted = Cycles.accept<system>(Nat.min(available, 10_000_000));
    { accepted = Nat64.fromNat(accepted) };
  };

  // Returns the default account identifier of this canister.
  public query func canister_memory() : async Nat {
    return Prim.rts_memory_size() + Prim.rts_heap_size();
  };

  public query ({ caller }) func profile() : async ?UserInfo {
    //
    // if (isMove()) {
    //   throw(Error.reject("Permanent:" # new_canister_id));
    // }

    switch (users_v3.get(caller)) {
      case (?store) {
        return ?toUserInfo(caller, store);
      };
      case (_) {
        return null;
      };
    };
  };

  //login
  public shared ({ caller }) func login() : async UserInfo {
    switch (users_v3.get(caller)) {
      case null {
        let user : UserV3 = {
          var avatar = "";
          var email = "";
          var nft = null;
          planets = Queue.empty();
          subscribes = Queue.empty();
          collections = Queue.empty();
          attributes = Queue.empty();
          created = Time.now();
        };
        users_v3.put(caller, user);
        return toUserInfo(caller, user);
      };
      case (?store) {
        return toUserInfo(caller, store);
      };
    };
  };

  //login
  public shared ({ caller }) func login_proxy(proxyuser : Principal) : async UserInfo {
    assert (caller == owner);
    switch (users_v3.get(proxyuser)) {
      case null {
        let user : UserV3 = {
          var avatar = "";
          var email = "";
          var nft = null;
          planets = Queue.empty();
          subscribes = Queue.empty();
          collections = Queue.empty();
          attributes = Queue.empty();
          created = Time.now();
        };
        users_v3.put(proxyuser, user);
        return toUserInfo(proxyuser, user);
      };
      case (?store) {
        return toUserInfo(proxyuser, store);
      };
    };
  };

  public shared ({ caller }) func add_attribute(p : Attribute) : async Bool {
    switch (users_v3.get(caller)) {
      case (?store) {
        ignore Queue.removeOne(store.attributes, Types.eqAttribute(p.key));
        ignore Queue.pushBack(store.attributes, p);
        return true;
      };
      case (_) {
        return false;
      };
    };
  };

  public shared ({ caller }) func set_email(p : Text) : async Bool {
    switch (users_v3.get(caller)) {
      case (?store) {
        store.email := p;
        return true;
      };
      case (_) {
        return false;
      };
    };
  };

  public query ({ caller }) func get_email() : async Text {
    switch (users_v3.get(caller)) {
      case (?store) {
        return store.email;
      };
      case (_) {
        return "";
      };
    };
  };

  public shared ({ caller }) func set_avatar(p : Text) : async Bool {
    switch (users_v3.get(caller)) {
      case (?store) {
        store.avatar := p;
        return true;
      };
      case (_) {
        return false;
      };
    };
  };

  public query ({ caller }) func get_avatar(user : ?Principal) : async Text {
    let uid = switch (user) {
      case (?id) { id };
      case (_) { caller };
    };
    switch (users_v3.get(uid)) {
      case (?store) {
        return store.avatar;
      };
      case (_) {
        return "";
      };
    };
  };

  // Add Planet
  public shared ({ caller }) func create_planet(args : PlanetArgs) : async Helper.CreatePlanetResp {
    switch (users_v3.get(caller)) {
      case (?store) {
        let helperActor : Helper.LaunchHelper = actor (Principal.toText(launchHelperID));
        let ret = await helperActor.createPlanet({
          owner = caller;
          name = args.name;
          avatar = args.avatar;
          desc = args.desc;
          code = args.code;
        });
        switch (ret) {
          case (#Ok(val)) {
            ignore Queue.pushBack(store.planets, val.id);
          };
          case (_) {};
        };
        ret;
      };
      case (_) {
        return #Err("Error: not the register user in this canister");
      };
    };
  };

  public shared ({ caller }) func on_planet_msg(planet : Principal, msg : PlanetMsg) : async Bool {
    assert (caller == owner);
    switch (msg.msg_type) {
      case (#subscribe) {
        return await add_subscribe(msg.user, planet);
      };
      case (#unsubscribe) {
        return await remove_subscribe(msg.user, planet);
      };
      case (#add) {
        return await add_planet(msg.user, planet);
      };
      case (#remove) {
        return await remove_planet(msg.user, planet);
      };
    };
    return false;
  };

  //
  public query ({ caller }) func get_attibutes() : async ?[Attribute] {
    switch (users_v3.get(caller)) {
      case null {
        return null;
      };
      case (?store) {
        return ?Queue.toArray<Attribute>(store.attributes);
      };
    };
  };

  public query ({ caller }) func get_attibute_by_key(key : Text) : async ?Attribute {
    switch (users_v3.get(caller)) {
      case null {
        return null;
      };
      case (?store) {
        switch (Queue.find(store.attributes, Types.eqAttribute(key))) {
          case (?attr) {
            return ?attr;
          };
          case (_) {
            return null;
          };
        };
      };
    };
  };

  //Acquire the planets you own
  public query ({ caller }) func get_planets() : async ?[Principal] {
    switch (users_v3.get(caller)) {
      case null {
        return null;
      };
      case (?store) {
        return ?Queue.toArray<Principal>(store.planets);
      };
    };
  };

  //Get a subscription to the planet
  public query ({ caller }) func get_subscribes() : async ?[Principal] {
    switch (users_v3.get(caller)) {
      case null {
        return null;
      };
      case (?store) {
        return ?Queue.toArray<Principal>(store.subscribes);
      };
    };
  };

  public query ({ caller }) func get_collections(req : QueryCommonReq) : async QueryCollectionResp {
    switch (users_v3.get(caller)) {
      case null {
        return { page = req.page; total = 0; hasmore = false; data = [] };
      };
      case (?store) {
        let res = limitCollections(caller, req, store.collections);
        return {
          page = req.page;
          total = res.0;
          hasmore = res.1;
          data = res.2;
        };
      };
    };
  };

  public shared ({ caller }) func add_collection(canister_id : Principal, article_id : Text) : async Bool {
    switch (users_v3.get(caller)) {
      case null {
        return false;
      };
      case (?store) {
        switch (Queue.find(store.collections, Types.eqCollection(canister_id, article_id))) {
          case (?cl) {
            return true;
          };
          case (_) {
            ignore Queue.pushFront({ canister_id = canister_id; article_id = article_id }, store.collections);
            return true;
          };
        };
      };
    };
    return false;
  };

  public shared ({ caller }) func remove_collection(canister_id : Principal, article_id : Text) : async Bool {
    switch (users_v3.get(caller)) {
      case null {
        return false;
      };
      case (?store) {
        let res = Queue.removeOne(store.collections, Types.eqCollection(canister_id, article_id));
        switch (res) {
          case (?cl) {
            return true;
          };
          case (_) {};
        };
      };
    };
    return false;
  };

  private func toUserInfo(caller : Principal, user : UserV3) : UserInfo {
    return {
      pid = caller;
      avatar = user.avatar;
      nft = user.nft;
      email = user.email;
      created = user.created / 1_000_000;
    };
  };

  //Check whether the user is registered
  private func hasUser(principal : Principal) : Bool {
    Option.isSome(users_v3.get(principal));
  };

  public query ({ caller }) func whoami() : async Principal {
    return caller;
  };

  system func preupgrade() {
    stable_users_v3 := Iter.toArray(users_v3.entries());
  };

  system func postupgrade() {
    users_v3 := TrieMap.fromEntries<Principal, UserV3>(stable_users_v3.vals(), Principal.equal, Principal.hash);
    stable_users_v3 := [];

    // for ((pid, user) in stable_users_v2.vals()) {
    //   let newuser : UserV3 = {
    //     var avatar = user.avatar;
    //     var email = user.email;
    //     var nft = user.nft;
    //     planets = user.planets;
    //     subscribes = user.subscribes;
    //     collections = Queue.empty();
    //     attribute = user.attribute;
    //     created = user.created;
    //   };
    //   users_v3.put(pid, newuser);
    // };
    // stable_users_v2 := [];
  };

  //setwriter or owner change
  private func add_planet(user : Principal, pid : Principal) : async Bool {
    switch (users_v3.get(user)) {
      case (?store) {
        switch (Queue.find<Principal>(store.planets, func(p : Principal) { Principal.equal(p, pid) })) {
          case (?ps) { return false };
          case (_) {
            // verify owner or writers
            let planet : Planet.PlanetActor = actor (Principal.toText(pid));
            let res = await planet.verifyOwnerWriter(user);
            if (res) {
              ignore Queue.pushBack(store.planets, pid);
              return true;
            };
            return false;
          };
        };
      };
      case (_) {};
    };
    return false;
  };

  //setwriter or owner change
  private func remove_planet(user : Principal, pid : Principal) : async Bool {
    switch (users_v3.get(user)) {
      case null { return false };
      case (?store) {
        // verify owner or writers
        let planet : Planet.PlanetActor = actor (Principal.toText(pid));
        let res = await planet.verifyOwnerWriter(user);
        if (not res) {
          ignore Queue.remove<Principal>(
            store.planets,
            func(p : Principal) { Principal.equal(p, pid) },
          );
          return true;
        };
      };
    };
    false;
  };

  //Add subscribe
  private func add_subscribe(user : Principal, pid : Principal) : async Bool {
    switch (users_v3.get(user)) {
      case (?store) {
        switch (Queue.find<Principal>(store.subscribes, func(p : Principal) { Principal.equal(p, pid) })) {
          case (?ps) { return false };
          case (_) {
            // verify subcribers
            let planet : Planet.PlanetActor = actor (Principal.toText(pid));
            let res = await planet.verifySubcriber(user);
            if (res) {
              ignore Queue.pushBack(store.subscribes, pid);
              return true;
            };
            return false;
          };
        };
      };
      case (_) {};
    };
    return false;
  };

  //
  private func remove_subscribe(user : Principal, pid : Principal) : async Bool {
    switch (users_v3.get(user)) {
      case null { return false };
      case (?store) {
        // verify subcribers
        let planet : Planet.PlanetActor = actor (Principal.toText(pid));
        let res = await planet.verifySubcriber(user);
        if (not res) {
          ignore Queue.remove<Principal>(
            store.subscribes,
            func(p : Principal) { Principal.equal(p, pid) },
          );
          return true;
        };
      };
    };
    false;
  };

  private func limitCollections(_caller : Principal, req : QueryCommonReq, q : Queue.Queue<Collection>) : (Int, Bool, [Collection]) {
    var data = Buffer.Buffer<Collection>(0);
    let pagesize = checkPageSize(req.page, req.size);
    let size = pagesize.1;
    var start = (pagesize.0 - 1) * size;
    var hasmore = false;
    var total = 0;

    var iter : Iter.Iter<Collection> = Queue.toIter(q);

    Iter.iterate(
      iter,
      func(x : Collection, _idx : Int) {
        if (total >= start and total < start + size) {
          data.add(x);
        };
        total := total + 1;
      },
    );
    if (total >= start + size) {
      hasmore := true;
    };
    return (total, hasmore, Buffer.toArray(data));
  };

  private func checkPageSize(p : Nat, s : Nat) : (Int, Int) {
    var page : Int = p;
    if (page < 1) {
      page := 1;
    };
    var size : Int = s;
    if (size > 50) {
      size := 50;
      // limit max page size
    } else if (size < 1) {
      size := 10;
    };
    return (page, size);
  };
};
