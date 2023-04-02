use coerce::actor::{ActorId, IntoActor, IntoActorId, Actor};
 use coerce::actor::context::ActorContext;
 use coerce::actor::message::{Message, Handler};
 use coerce::actor::system::ActorSystem;

 use async_trait::async_trait;
 use tokio::sync::oneshot::{channel, Sender};

 struct ParentActor {
    child_count: usize,
    completed_actors: usize,
    on_work_completed: Option<Sender<usize>>,
 }

 struct ChildActor;

 #[tokio::main]
 pub async fn main() {
    let system = ActorSystem::new();
    let (tx, rx) = channel();

    const TOTAL_CHILD_ACTORS: usize = 10;

    let actor = ParentActor {
            child_count: TOTAL_CHILD_ACTORS,
            completed_actors: 0,
            on_work_completed: Some(tx),
         }
        .into_actor(Some("parent"), &system)
        .await
        .unwrap();

    let completed_actors = rx.await.ok();
    assert_eq!(completed_actors, Some(TOTAL_CHILD_ACTORS))
 }

 #[async_trait]
 impl Actor for ParentActor {
    async fn started(&mut self, ctx: &mut ActorContext) {
        for i in 0..self.child_count {
            ctx.spawn(format!("child-{}", i).into_actor_id(), ChildActor).await.unwrap();
        }
    }

    async fn on_child_stopped(&mut self, id: &ActorId, ctx: &mut ActorContext) {
        println!("child actor (id={}) stopped", ctx.id());

        self.completed_actors += 1;

        if ctx.supervised_count() == 0 && self.completed_actors == self.child_count {
            println!("all child actors finished, stopping ParentActor");

            if let Some(on_work_completed) = self.on_work_completed.take() {
                let _ = on_work_completed.send(self.completed_actors);
            }

            ctx.stop(None);
        }
    }
 }

 struct Finished;

 impl Message for Finished {
    type Result = ();
 }

 #[async_trait]
 impl Actor for ChildActor {
    async fn started(&mut self, ctx: &mut ActorContext) {
        println!("child actor (id={}) running", ctx.id());

        // simulate some work that takes 5 milliseconds
        let _ = self.actor_ref(ctx)
                    .scheduled_notify(Finished, std::time::Duration::from_millis(5));
    }

    async fn stopped(&mut self, ctx: &mut ActorContext) {
        println!("child actor (id={}) finished", ctx.id());
    }
 }

 #[async_trait]
 impl Handler<Finished> for ChildActor {
     async fn handle(&mut self, message: Finished, ctx: &mut ActorContext)  {
        ctx.stop(None);
     }
 }