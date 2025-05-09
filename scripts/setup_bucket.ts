import { createClient } from "@supabase/supabase-js";
import dotenv from "dotenv";

dotenv.config();

async function setupSupabaseBucket() {
  const supabaseUrl = process.env.SUPABASE_URL;
  const supabaseServiceRoleKey = process.env.SUPABASE_SERVICE_ROLE_KEY;
  const bucketName = process.env.SUPABASE_BUCKET_NAME;

  if (!supabaseUrl || !supabaseServiceRoleKey || !bucketName) {
    console.error(
      "Error: SUPABASE_URL, SUPABASE_SERVICE_ROLE_KEY, and SUPABASE_BUCKET_NAME must be set in your .env file."
    );
    process.exit(1);
  }

  const supabase = createClient(supabaseUrl, supabaseServiceRoleKey);

  try {
    // Check if the bucket already exists
    const { data: existingBuckets, error: listError } =
      await supabase.storage.listBuckets();

    if (listError) {
      console.error("Error listing buckets:", listError.message);
      return;
    }

    const bucketExists = existingBuckets?.some(
      (bucket) => bucket.name === bucketName
    );

    if (bucketExists) {
      console.log(`Bucket "${bucketName}" already exists. Skipping creation.`);
    } else {
      console.log(`Bucket "${bucketName}" does not exist. Creating...`);
      const { data, error } = await supabase.storage.createBucket(bucketName, {
        public: true, // Set to true if you want the bucket to be public
      });

      if (error) {
        console.error(`Error creating bucket "${bucketName}":`, error.message);
      } else {
        console.log(`Bucket "${bucketName}" created successfully:`, data);
      }
    }
  } catch (error: any) {
    console.error("An unexpected error occurred:", error.message);
  }
}

setupSupabaseBucket();
