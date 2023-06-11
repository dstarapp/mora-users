import Buffer "mo:base/Buffer";
import HashMap "mo:base/HashMap";
import Nat64 "mo:base/Nat64";
import Text "mo:base/Text";
import Queue "mo:mutable-queue/Queue";
import Bool "mo:base/Bool";

module {

    public type UserV3 = {
        var avatar : Text;
        var nft : ?NFT;
        var email : Text;
        planets : Queue.Queue<Principal>;
        subscribes : Queue.Queue<Principal>;
        collections : Queue.Queue<Collection>;
        attributes : Queue.Queue<Attribute>;
        created : Int;
    };

    public type UserInfo = {
        pid : Principal;
        avatar : Text;
        nft : ?NFT;
        email : Text;
        created : Int;
    };

    public type Attribute = {
        key : Text;
        value : Text;
    };

    public type NFT = {
        canister_id : Principal;
        standard : Text;
        token_index : Text;
    };

    public type Collection = {
        canister_id : Principal;
        article_id : Text;
    };

    public type PlanetArgs = {
        name : Text;
        desc : Text;
        avatar : Text;
        code : Text;
    };

    public type PlanetMsgType = {
        #subscribe;
        #unsubscribe;
        #add;
        #remove;
    };

    public type PlanetMsg = {
        msg_type : PlanetMsgType;
        user : Principal;
        data : ?Blob;
    };

    public type QueryCommonReq = {
        page : Nat;
        size : Nat;
    };

    public type QueryCollectionResp = {
        page : Nat;
        total : Int;
        hasmore : Bool;
        data : [Collection];
    };

    public type UserInfoData = {
        pid : Principal;
        avatar : Text;
        nft : ?NFT;
        email : Text;
        created : Int;
        planets : [Principal];
        subscribes : [Principal];
        collections : [Collection];
    };

    public func eqCollection(canister_id : Principal, article_id : Text) : Collection -> Bool {
        func(x : Collection) : Bool { x.canister_id == canister_id and x.article_id == article_id };
    };

    public func eqAttribute(key : Text) : Attribute -> Bool {
        func(x : Attribute) : Bool { x.key == key };
    };
};
